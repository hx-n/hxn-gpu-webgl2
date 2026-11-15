//! The backend for a threaded Ash build. Each call is encoded onto the wire (`wire.rs`, generated
//! from WebGL's IDL by x-idl) into one batch, which the browser agent runs on the web page's WebGL2
//! context in a Worker of its own; the browser agent is `page/gpu_webgl2.mjs`, which the web page
//! starts when `Context.create` asks for it through `ash_host_agent`. Objects are kept by the
//! browser agent under handles this library picks, so making one needs no round trip; a batch goes
//! out when a call needs an answer. `completed`'s promise is a reply record in the program's
//! shared memory, which Ash's watch on the mailbox settles into an hxn future on the program's
//! thread.

// Generated for all of WebGL's IDL, of which this backend uses a part.
#[allow(dead_code, clippy::all)]
mod wire;

use std::sync::atomic::{AtomicI32, AtomicU32, Ordering::SeqCst};
use std::sync::{Mutex, MutexGuard, Once};

use hxn_future::{Completer, Future};
use wire::{Bytes, Decode, Decoder, Encoder, Handle, Mailbox};

use crate::{Kind, gl};

/// The WebGL2 context, as the browser agent keeps it.
const GL: Handle = Handle(1);

/// The context's canvas.
const CANVAS: Handle = Handle(2);

/// The browser agent's helpers (`tools/wire/idl/agent.idl`).
const AGENT_HELPERS: Handle = Handle(3);

/// The handles the browser agent holds before this library makes any.
const ROOTS: u32 = 3;

/// The name of the browser agent, and of its shim beside the web page: `gpu_webgl2.mjs`.
const AGENT: &str = "gpu_webgl2";

static MAILBOX: Mailbox = Mailbox::new();

unsafe extern "C" {
    fn ash_host_agent(name: *const u8, name_len: u32, address: u32) -> i32;
    fn ash_host_watch(
        word: *const u32,
        handler: unsafe extern "C" fn(*mut std::ffi::c_void),
        context: *mut std::ffi::c_void,
    ) -> *const u32;
}

/// Run by Ash on the program's thread whenever the browser agent has settled another reply.
unsafe extern "C" fn settled(_: *mut std::ffi::c_void) {
    settle_answered();
}

/// A reply record: the browser agent's answer to one command, in the program's memory, and the
/// buffer that takes what it carries. State 0 is pending, 1 done, 2 rejected, 3 too small.
#[repr(C)]
struct Reply {
    state: AtomicI32,
    len: AtomicU32,
    address: u32,
    cap: u32,
}

/// A reply with a buffer of its own, boxed so its address stays put until it settles.
struct Answer {
    reply: Reply,
    buffer: Box<[u8]>,
}

impl Answer {
    fn new(capacity: usize) -> Box<Answer> {
        let buffer = vec![0u8; capacity].into_boxed_slice();
        let address = buffer.as_ptr() as usize as u32;
        Box::new(Answer {
            reply: Reply {
                state: AtomicI32::new(0),
                len: AtomicU32::new(0),
                address,
                cap: capacity as u32,
            },
            buffer,
        })
    }

    fn at(&self) -> u32 {
        &self.reply as *const Reply as usize as u32
    }

    fn state(&self) -> i32 {
        self.reply.state.load(SeqCst)
    }

    fn bytes(&self) -> &[u8] {
        let len = (self.reply.len.load(SeqCst) as usize).min(self.buffer.len());
        &self.buffer[..len]
    }

    fn decode<T: Decode>(&self) -> Option<T> {
        if self.state() != 1 {
            return None;
        }
        T::decode(&mut Decoder::new(self.bytes()))
    }

    /// What a rejection's reply carries.
    fn message(&self) -> String {
        if self.state() == 3 {
            return "the browser agent's answer is too long to carry".into();
        }
        String::from_utf8_lossy(self.bytes()).into_owned()
    }
}

struct State {
    commands: Encoder,
    /// Bytes the batch's commands point at, kept until it is sent.
    staged: Vec<Box<[u8]>>,
    next: u32,
    /// `completed` futures and the replies that settle them.
    pending: Vec<(Box<Answer>, Completer<()>)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    commands: Encoder { bytes: Vec::new() },
    staged: Vec::new(),
    next: ROOTS,
    pending: Vec::new(),
});

fn state() -> MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl State {
    fn handle(&mut self) -> Handle {
        self.next += 1;
        Handle(self.next)
    }

    /// Hands the batch to the browser agent and waits until it has run it.
    fn flush(&mut self) {
        MAILBOX.send(&self.commands.bytes);
        self.commands.bytes.clear();
        self.staged.clear();
    }

    /// `bytes`, copied until the batch is sent, as the wire reads them.
    fn stage(&mut self, bytes: &[u8]) -> Bytes {
        let copy: Box<[u8]> = bytes.into();
        let staged = lent(&copy);
        self.staged.push(copy);
        staged
    }

    /// Asks with `ask`, sends the batch, and decodes the answer.
    fn ask<T: Decode>(
        &mut self,
        capacity: usize,
        ask: impl FnOnce(&mut Encoder, u32),
    ) -> Option<T> {
        let answer = Answer::new(capacity);
        ask(&mut self.commands, answer.at());
        self.flush();
        answer.decode()
    }
}

/// `bytes` where they are, for a command run before they go away.
fn lent(bytes: &[u8]) -> Bytes {
    Bytes {
        address: bytes.as_ptr() as usize as u32,
        len: bytes.len() as u32,
    }
}

fn some(name: u32) -> Option<Handle> {
    (name != 0).then_some(Handle(name))
}

/// Settles every `completed` future the browser agent has answered. Run on the program's thread
/// by Ash's watch.
fn settle_answered() {
    let mut s = state();
    let mut i = 0;
    while i < s.pending.len() {
        if s.pending[i].0.state() == 0 {
            i += 1;
            continue;
        }
        let (answer, completer) = s.pending.swap_remove(i);
        if answer.state() == 1 {
            completer.resolve(());
        } else {
            completer.reject(answer.message());
        }
    }
}

/// Starts the browser agent, and has Ash settle what it answers, once.
fn start() -> Result<(), String> {
    static STARTED: Once = Once::new();
    static AGENT_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    STARTED.call_once(|| {
        // Watched before any request is sent, so no reply is missed. Ash runs the handler on
        // the program's thread while it waits; the browser agent adds one to the word Ash
        // answers, and notifies it, after settling a reply, which wakes that wait.
        let wake = unsafe {
            ash_host_watch(
                MAILBOX.settled.as_ptr().cast_const().cast(),
                settled,
                std::ptr::null_mut(),
            )
        };
        if wake.is_null() {
            return;
        }
        MAILBOX.wake.store(wake as usize as u32, SeqCst);
        let address = &MAILBOX as *const Mailbox as usize as u32;
        let ran = unsafe { ash_host_agent(AGENT.as_ptr(), AGENT.len() as u32, address) };
        if ran == 1 {
            AGENT_RUNNING.store(true, SeqCst);
        }
    });
    if AGENT_RUNNING.load(SeqCst) {
        Ok(())
    } else {
        Err(format!(
            "no web page started the WebGL2 browser agent: run the program in the web page Ash writes, served by `ash serve`, with {AGENT}.mjs beside it"
        ))
    }
}

/// The web page's WebGL2 context, which the browser agent holds.
pub struct Gl {}

impl Gl {
    pub fn open() -> Result<Gl, String> {
        start()?;
        // The context's version, asked through a helper that says why there is no context.
        let answer = Answer::new(1024);
        let mut s = state();
        s.commands
            .web_gl2agent_parameter(AGENT_HELPERS, answer.at(), &GL, &gl::VERSION);
        s.flush();
        match answer.state() {
            1 => Ok(Gl {}),
            _ => Err(answer.message()),
        }
    }

    pub fn set_size(&self, width: u32, height: u32) {
        let mut s = state();
        s.commands
            .offscreen_canvas_set_width(CANVAS, &(width as u64));
        s.commands
            .offscreen_canvas_set_height(CANVAS, &(height as u64));
    }

    pub fn drawing_buffer_size(&self) -> (i32, i32) {
        let mut s = state();
        let width = Answer::new(16);
        let height = Answer::new(16);
        s.commands
            .web_gl2rendering_context_get_drawing_buffer_width(GL, width.at());
        s.commands
            .web_gl2rendering_context_get_drawing_buffer_height(GL, height.at());
        s.flush();
        (width.decode().unwrap_or(0), height.decode().unwrap_or(0))
    }

    pub fn parameter(&self, name: u32) -> String {
        state()
            .ask(4096, |c, reply| {
                c.web_gl2agent_parameter(AGENT_HELPERS, reply, &GL, &name)
            })
            .unwrap_or_default()
    }

    pub fn extensions(&self) -> Vec<String> {
        state()
            .ask::<Option<Vec<String>>>(16384, |c, reply| {
                c.web_gl2rendering_context_get_supported_extensions(GL, reply)
            })
            .flatten()
            .unwrap_or_default()
    }

    pub fn error(&self) -> u32 {
        state()
            .ask(16, |c, reply| {
                c.web_gl2rendering_context_get_error(GL, reply)
            })
            .unwrap_or(0)
    }

    pub fn create(&self, kind: Kind) -> Result<u32, String> {
        let mut s = state();
        let name = s.handle();
        let c = &mut s.commands;
        match kind {
            Kind::Buffer => c.web_gl2rendering_context_create_buffer(GL, name),
            Kind::Texture => c.web_gl2rendering_context_create_texture(GL, name),
            Kind::Sampler => c.web_gl2rendering_context_create_sampler(GL, name),
            Kind::VertexArray => c.web_gl2rendering_context_create_vertex_array(GL, name),
            Kind::Framebuffer => c.web_gl2rendering_context_create_framebuffer(GL, name),
            Kind::Program => c.web_gl2rendering_context_create_program(GL, name),
            Kind::Shader => unreachable!("a shader is made with create_shader"),
        }
        Ok(name.0)
    }

    pub fn create_shader(&self, kind: u32) -> Result<u32, String> {
        let mut s = state();
        let name = s.handle();
        let made: Option<bool> = s.ask(16, |c, reply| {
            c.web_gl2rendering_context_create_shader(GL, name, reply, &kind)
        });
        match made {
            Some(true) => Ok(name.0),
            _ => Err("WebGL could not create a shader".into()),
        }
    }

    pub fn delete(&self, kind: Kind, name: u32) {
        let mut s = state();
        let c = &mut s.commands;
        let object = &Some(Handle(name));
        match kind {
            Kind::Buffer => c.web_gl2rendering_context_delete_buffer(GL, object),
            Kind::Texture => c.web_gl2rendering_context_delete_texture(GL, object),
            Kind::Sampler => c.web_gl2rendering_context_delete_sampler(GL, object),
            Kind::VertexArray => c.web_gl2rendering_context_delete_vertex_array(GL, object),
            Kind::Framebuffer => c.web_gl2rendering_context_delete_framebuffer(GL, object),
            Kind::Program => c.web_gl2rendering_context_delete_program(GL, object),
            Kind::Shader => c.web_gl2rendering_context_delete_shader(GL, object),
        }
        c.release(Handle(name));
    }

    pub fn bind_buffer(&self, target: u32, name: u32) {
        state()
            .commands
            .web_gl2rendering_context_bind_buffer(GL, &target, &some(name));
    }

    pub fn bind_buffer_base(&self, target: u32, index: u32, name: u32) {
        state().commands.web_gl2rendering_context_bind_buffer_base(
            GL,
            &target,
            &index,
            &some(name),
        );
    }

    pub fn buffer_data_size(&self, target: u32, size: i32, usage: u32) {
        state()
            .commands
            .web_gl2rendering_context_buffer_data(GL, &target, &(size as i64), &usage);
    }

    pub fn buffer_data(&self, target: u32, bytes: &[u8], usage: u32) {
        let mut s = state();
        let staged = s.stage(bytes);
        s.commands
            .web_gl2rendering_context_buffer_data_2(GL, &target, &Some(staged), &usage);
    }

    pub fn buffer_sub_data(&self, target: u32, offset: i32, bytes: &[u8]) {
        let mut s = state();
        let staged = s.stage(bytes);
        s.commands
            .web_gl2rendering_context_buffer_sub_data(GL, &target, &(offset as i64), &staged);
    }

    /// Fills `bytes` in place: the browser agent writes them into the program's memory before
    /// the batch counts as run.
    pub fn get_buffer_sub_data(&self, target: u32, offset: i32, bytes: &mut [u8]) {
        let mut s = state();
        s.commands.web_gl2rendering_context_get_buffer_sub_data(
            GL,
            &target,
            &(offset as i64),
            &lent(bytes),
            &None,
            &None,
        );
        s.flush();
    }

    pub fn bind_texture(&self, target: u32, name: u32) {
        state()
            .commands
            .web_gl2rendering_context_bind_texture(GL, &target, &some(name));
    }

    pub fn active_texture(&self, unit: u32) {
        state()
            .commands
            .web_gl2rendering_context_active_texture(GL, &unit);
    }

    pub fn tex_storage_2d(&self, format: u32, width: i32, height: i32) {
        state().commands.web_gl2rendering_context_tex_storage2d(
            GL,
            &gl::TEXTURE_2D,
            &1,
            &format,
            &width,
            &height,
        );
    }

    /// WebGL takes 8-bit pixels as the bytes they are, and any other type only as a typed array
    /// of that type, so those go through a pixel unpack buffer, which takes bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn tex_sub_image_2d(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: u32,
        kind: u32,
        pixels: &[u8],
    ) -> Result<(), String> {
        let mut s = state();
        let staged = s.stage(pixels);
        if kind == gl::UNSIGNED_BYTE {
            s.commands.web_gl2rendering_context_tex_sub_image2d(
                GL,
                &gl::TEXTURE_2D,
                &0,
                &x,
                &y,
                &width,
                &height,
                &format,
                &kind,
                &Some(staged),
            );
            return Ok(());
        }
        let unpack = s.handle();
        let c = &mut s.commands;
        c.web_gl2rendering_context_create_buffer(GL, unpack);
        c.web_gl2rendering_context_bind_buffer(GL, &gl::PIXEL_UNPACK_BUFFER, &Some(unpack));
        c.web_gl2rendering_context_buffer_data_2(
            GL,
            &gl::PIXEL_UNPACK_BUFFER,
            &Some(staged),
            &gl::STREAM_DRAW,
        );
        c.web_gl2rendering_context_tex_sub_image2d_3(
            GL,
            &gl::TEXTURE_2D,
            &0,
            &x,
            &y,
            &width,
            &height,
            &format,
            &kind,
            &0,
        );
        c.web_gl2rendering_context_bind_buffer(GL, &gl::PIXEL_UNPACK_BUFFER, &None);
        c.web_gl2rendering_context_delete_buffer(GL, &Some(unpack));
        c.release(unpack);
        Ok(())
    }

    pub fn pixel_store(&self, name: u32, value: i32) {
        state()
            .commands
            .web_gl2rendering_context_pixel_storei(GL, &name, &value);
    }

    pub fn sampler_parameter(&self, sampler: u32, name: u32, value: i32) {
        state()
            .commands
            .web_gl2rendering_context_sampler_parameteri(GL, &Handle(sampler), &name, &value);
    }

    pub fn bind_sampler(&self, unit: u32, sampler: u32) {
        state()
            .commands
            .web_gl2rendering_context_bind_sampler(GL, &unit, &some(sampler));
    }

    pub fn shader_source(&self, shader: u32, source: &str) {
        state().commands.web_gl2rendering_context_shader_source(
            GL,
            &Handle(shader),
            &source.to_owned(),
        );
    }

    pub fn compile_shader(&self, shader: u32) {
        state()
            .commands
            .web_gl2rendering_context_compile_shader(GL, &Handle(shader));
    }

    pub fn compiled(&self, shader: u32) -> bool {
        state()
            .ask(16, |c, reply| {
                c.web_gl2agent_compiled(AGENT_HELPERS, reply, &GL, &Handle(shader))
            })
            .unwrap_or(false)
    }

    pub fn shader_log(&self, shader: u32) -> String {
        state()
            .ask::<Option<String>>(65536, |c, reply| {
                c.web_gl2rendering_context_get_shader_info_log(GL, reply, &Handle(shader))
            })
            .flatten()
            .unwrap_or_default()
    }

    pub fn attach_shader(&self, program: u32, shader: u32) {
        state().commands.web_gl2rendering_context_attach_shader(
            GL,
            &Handle(program),
            &Handle(shader),
        );
    }

    pub fn detach_shader(&self, program: u32, shader: u32) {
        state().commands.web_gl2rendering_context_detach_shader(
            GL,
            &Handle(program),
            &Handle(shader),
        );
    }

    pub fn link_program(&self, program: u32) {
        state()
            .commands
            .web_gl2rendering_context_link_program(GL, &Handle(program));
    }

    pub fn linked(&self, program: u32) -> bool {
        state()
            .ask(16, |c, reply| {
                c.web_gl2agent_linked(AGENT_HELPERS, reply, &GL, &Handle(program))
            })
            .unwrap_or(false)
    }

    pub fn program_log(&self, program: u32) -> String {
        state()
            .ask::<Option<String>>(65536, |c, reply| {
                c.web_gl2rendering_context_get_program_info_log(GL, reply, &Handle(program))
            })
            .flatten()
            .unwrap_or_default()
    }

    pub fn use_program(&self, program: u32) {
        state()
            .commands
            .web_gl2rendering_context_use_program(GL, &some(program));
    }

    pub fn uniform_block_index(&self, program: u32, name: &str) -> u32 {
        state()
            .ask(16, |c, reply| {
                c.web_gl2rendering_context_get_uniform_block_index(
                    GL,
                    reply,
                    &Handle(program),
                    &name.to_owned(),
                )
            })
            .unwrap_or(gl::INVALID_INDEX)
    }

    pub fn uniform_block_binding(&self, program: u32, index: u32, binding: u32) {
        state()
            .commands
            .web_gl2rendering_context_uniform_block_binding(GL, &Handle(program), &index, &binding);
    }

    pub fn sampler_unit(&self, program: u32, name: &str, unit: i32) -> bool {
        state()
            .ask(16, |c, reply| {
                c.web_gl2agent_sampler_unit(
                    AGENT_HELPERS,
                    reply,
                    &GL,
                    &Handle(program),
                    &name.to_owned(),
                    &unit,
                )
            })
            .unwrap_or(false)
    }

    pub fn bind_vertex_array(&self, name: u32) {
        state()
            .commands
            .web_gl2rendering_context_bind_vertex_array(GL, &some(name));
    }

    pub fn vertex_attrib_pointer(
        &self,
        location: u32,
        size: i32,
        kind: u32,
        normalized: bool,
        stride: i32,
        offset: i32,
    ) {
        state()
            .commands
            .web_gl2rendering_context_vertex_attrib_pointer(
                GL,
                &location,
                &size,
                &kind,
                &normalized,
                &stride,
                &(offset as i64),
            );
    }

    pub fn vertex_attrib_i_pointer(
        &self,
        location: u32,
        size: i32,
        kind: u32,
        stride: i32,
        offset: i32,
    ) {
        state()
            .commands
            .web_gl2rendering_context_vertex_attrib_i_pointer(
                GL,
                &location,
                &size,
                &kind,
                &stride,
                &(offset as i64),
            );
    }

    pub fn enable_vertex_attrib_array(&self, location: u32) {
        state()
            .commands
            .web_gl2rendering_context_enable_vertex_attrib_array(GL, &location);
    }

    pub fn vertex_attrib_divisor(&self, location: u32, divisor: u32) {
        state()
            .commands
            .web_gl2rendering_context_vertex_attrib_divisor(GL, &location, &divisor);
    }

    pub fn bind_framebuffer(&self, name: u32) {
        state().commands.web_gl2rendering_context_bind_framebuffer(
            GL,
            &gl::FRAMEBUFFER,
            &some(name),
        );
    }

    pub fn framebuffer_texture_2d(&self, attachment: u32, texture: u32) {
        state()
            .commands
            .web_gl2rendering_context_framebuffer_texture2d(
                GL,
                &gl::FRAMEBUFFER,
                &attachment,
                &gl::TEXTURE_2D,
                &some(texture),
                &0,
            );
    }

    pub fn check_framebuffer_status(&self) -> u32 {
        state()
            .ask(16, |c, reply| {
                c.web_gl2rendering_context_check_framebuffer_status(GL, reply, &gl::FRAMEBUFFER)
            })
            .unwrap_or(0)
    }

    pub fn viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        state()
            .commands
            .web_gl2rendering_context_viewport(GL, &x, &y, &width, &height);
    }

    pub fn scissor(&self, x: i32, y: i32, width: i32, height: i32) {
        state()
            .commands
            .web_gl2rendering_context_scissor(GL, &x, &y, &width, &height);
    }

    pub fn clear(&self, bits: u32, [r, g, b, a]: [f32; 4]) {
        let mut s = state();
        s.commands
            .web_gl2rendering_context_clear_color(GL, &r, &g, &b, &a);
        s.commands.web_gl2rendering_context_clear(GL, &bits);
    }

    pub fn draw_arrays_instanced(&self, mode: u32, first: i32, count: i32, instances: i32) {
        state()
            .commands
            .web_gl2rendering_context_draw_arrays_instanced(GL, &mode, &first, &count, &instances);
    }

    pub fn draw_elements_instanced(
        &self,
        mode: u32,
        count: i32,
        kind: u32,
        offset: i32,
        instances: i32,
    ) {
        state()
            .commands
            .web_gl2rendering_context_draw_elements_instanced(
                GL,
                &mode,
                &count,
                &kind,
                &(offset as i64),
                &instances,
            );
    }

    /// Fills `pixels` in place, in the same batch as the drawing before it.
    pub fn read_pixels(&self, x: i32, y: i32, width: i32, height: i32, pixels: &mut [u8]) {
        let mut s = state();
        s.commands.web_gl2rendering_context_read_pixels(
            GL,
            &x,
            &y,
            &width,
            &height,
            &gl::RGBA,
            &gl::UNSIGNED_BYTE,
            &Some(lent(pixels)),
        );
        s.flush();
    }

    /// Sends the batch, which the browser agent runs at once.
    pub fn flush(&self) {
        let mut s = state();
        s.commands.web_gl2rendering_context_flush(GL);
        s.flush();
    }

    pub fn finish(&self) {
        let mut s = state();
        s.commands.web_gl2rendering_context_finish(GL);
        s.flush();
    }

    pub fn completed(&self) -> Result<Future<()>, String> {
        let (completer, future) = hxn_future::pending();
        let mut s = state();
        let answer = Answer::new(1024);
        s.commands
            .web_gl2agent_completed(AGENT_HELPERS, answer.at(), &GL);
        s.pending.push((answer, completer));
        s.flush();
        Ok(future)
    }
}
