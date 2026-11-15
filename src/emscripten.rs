//! The backend over Emscripten's OpenGL ES 3.0 functions, which Emscripten implements on a WebGL2
//! context of the web page's `#canvas`: each call a wasm import into Emscripten's JavaScript.

use std::ffi::{c_char, c_int, c_void};

use hxn_future::Completer;

use crate::{Kind, gl};

const CANVAS: &std::ffi::CStr = c"#canvas";

const COMPILE_STATUS: u32 = 0x8B81;
const LINK_STATUS: u32 = 0x8B82;
const INFO_LOG_LENGTH: u32 = 0x8B84;
const EXTENSIONS: u32 = 0x1F03;
const NUM_EXTENSIONS: u32 = 0x821D;
const SYNC_GPU_COMMANDS_COMPLETE: u32 = 0x9117;
const TIMEOUT_EXPIRED: u32 = 0x911B;
const WAIT_FAILED: u32 = 0x911D;

/// `EmscriptenWebGLContextAttributes`.
#[repr(C)]
#[derive(Default)]
struct Attributes {
    alpha: bool,
    depth: bool,
    stencil: bool,
    antialias: bool,
    premultiplied_alpha: bool,
    preserve_drawing_buffer: bool,
    power_preference: c_int,
    fail_if_major_performance_caveat: bool,
    major_version: c_int,
    minor_version: c_int,
    enable_extensions_by_default: bool,
    explicit_swap_control: bool,
    proxy_context_to_main_thread: c_int,
    render_via_offscreen_back_buffer: bool,
}

unsafe extern "C" {
    fn emscripten_webgl_init_context_attributes(attributes: *mut Attributes);
    fn emscripten_webgl_create_context(
        target: *const c_char,
        attributes: *const Attributes,
    ) -> usize;
    fn emscripten_webgl_make_context_current(context: usize) -> c_int;
    fn emscripten_webgl_destroy_context(context: usize) -> c_int;
    fn emscripten_webgl_get_drawing_buffer_size(
        context: usize,
        width: *mut c_int,
        height: *mut c_int,
    ) -> c_int;
    fn emscripten_set_canvas_element_size(
        target: *const c_char,
        width: c_int,
        height: c_int,
    ) -> c_int;
    fn emscripten_set_timeout(
        callback: unsafe extern "C" fn(*mut c_void),
        milliseconds: f64,
        user: *mut c_void,
    ) -> c_int;

    fn glGetError() -> u32;
    fn glGetString(name: u32) -> *const c_char;
    fn glGetStringi(name: u32, index: u32) -> *const c_char;
    fn glGetIntegerv(name: u32, value: *mut c_int);
    fn glGenBuffers(n: c_int, names: *mut u32);
    fn glGenTextures(n: c_int, names: *mut u32);
    fn glGenSamplers(n: c_int, names: *mut u32);
    fn glGenVertexArrays(n: c_int, names: *mut u32);
    fn glGenFramebuffers(n: c_int, names: *mut u32);
    fn glCreateProgram() -> u32;
    fn glCreateShader(kind: u32) -> u32;
    fn glDeleteBuffers(n: c_int, names: *const u32);
    fn glDeleteTextures(n: c_int, names: *const u32);
    fn glDeleteSamplers(n: c_int, names: *const u32);
    fn glDeleteVertexArrays(n: c_int, names: *const u32);
    fn glDeleteFramebuffers(n: c_int, names: *const u32);
    fn glDeleteProgram(name: u32);
    fn glDeleteShader(name: u32);
    fn glBindBuffer(target: u32, name: u32);
    fn glBindBufferBase(target: u32, index: u32, name: u32);
    fn glBufferData(target: u32, size: isize, data: *const c_void, usage: u32);
    fn glBufferSubData(target: u32, offset: isize, size: isize, data: *const c_void);
    fn glGetBufferSubData(target: u32, offset: isize, size: isize, data: *mut c_void);
    fn glBindTexture(target: u32, name: u32);
    fn glActiveTexture(unit: u32);
    fn glTexStorage2D(target: u32, levels: c_int, format: u32, width: c_int, height: c_int);
    #[allow(clippy::too_many_arguments)]
    fn glTexSubImage2D(
        target: u32,
        level: c_int,
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
        format: u32,
        kind: u32,
        pixels: *const c_void,
    );
    fn glPixelStorei(name: u32, value: c_int);
    fn glSamplerParameteri(sampler: u32, name: u32, value: c_int);
    fn glBindSampler(unit: u32, sampler: u32);
    fn glShaderSource(
        shader: u32,
        count: c_int,
        sources: *const *const c_char,
        lengths: *const c_int,
    );
    fn glCompileShader(shader: u32);
    fn glGetShaderiv(shader: u32, name: u32, value: *mut c_int);
    fn glGetShaderInfoLog(shader: u32, size: c_int, length: *mut c_int, log: *mut c_char);
    fn glAttachShader(program: u32, shader: u32);
    fn glDetachShader(program: u32, shader: u32);
    fn glLinkProgram(program: u32);
    fn glGetProgramiv(program: u32, name: u32, value: *mut c_int);
    fn glGetProgramInfoLog(program: u32, size: c_int, length: *mut c_int, log: *mut c_char);
    fn glUseProgram(program: u32);
    fn glGetUniformBlockIndex(program: u32, name: *const c_char) -> u32;
    fn glUniformBlockBinding(program: u32, index: u32, binding: u32);
    fn glGetUniformLocation(program: u32, name: *const c_char) -> c_int;
    fn glUniform1i(location: c_int, value: c_int);
    fn glBindVertexArray(name: u32);
    fn glVertexAttribPointer(
        location: u32,
        size: c_int,
        kind: u32,
        normalized: u8,
        stride: c_int,
        offset: *const c_void,
    );
    fn glVertexAttribIPointer(
        location: u32,
        size: c_int,
        kind: u32,
        stride: c_int,
        offset: *const c_void,
    );
    fn glEnableVertexAttribArray(location: u32);
    fn glVertexAttribDivisor(location: u32, divisor: u32);
    fn glBindFramebuffer(target: u32, name: u32);
    fn glFramebufferTexture2D(
        target: u32,
        attachment: u32,
        texture_target: u32,
        texture: u32,
        level: c_int,
    );
    fn glCheckFramebufferStatus(target: u32) -> u32;
    fn glViewport(x: c_int, y: c_int, width: c_int, height: c_int);
    fn glScissor(x: c_int, y: c_int, width: c_int, height: c_int);
    fn glClearColor(r: f32, g: f32, b: f32, a: f32);
    fn glClear(bits: u32);
    fn glDrawArraysInstanced(mode: u32, first: c_int, count: c_int, instances: c_int);
    fn glDrawElementsInstanced(
        mode: u32,
        count: c_int,
        kind: u32,
        offset: *const c_void,
        instances: c_int,
    );
    #[allow(clippy::too_many_arguments)]
    fn glReadPixels(
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
        format: u32,
        kind: u32,
        pixels: *mut c_void,
    );
    fn glFlush();
    fn glFinish();
    fn glFenceSync(condition: u32, flags: u32) -> *mut c_void;
    fn glClientWaitSync(sync: *mut c_void, flags: u32, timeout: u64) -> u32;
    fn glDeleteSync(sync: *mut c_void);
}

fn text(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}

fn c_text(text: &str) -> std::ffi::CString {
    std::ffi::CString::new(text.replace('\0', "")).unwrap_or_default()
}

/// A log of `length` bytes that `fill` writes.
fn log(length: c_int, fill: impl FnOnce(c_int, *mut c_int, *mut c_char)) -> String {
    let mut bytes = vec![0u8; length.max(1) as usize];
    let mut written = 0;
    fill(
        bytes.len() as c_int,
        &mut written,
        bytes.as_mut_ptr().cast(),
    );
    bytes.truncate(written.max(0) as usize);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The page's WebGL2 context. The page has one thread, which every call is made on.
pub struct Gl {
    context: usize,
}

unsafe impl Send for Gl {}
unsafe impl Sync for Gl {}

impl Drop for Gl {
    fn drop(&mut self) {
        unsafe { emscripten_webgl_destroy_context(self.context) };
    }
}

/// A fence waited on from the page's event loop.
struct Fence {
    sync: *mut c_void,
    completer: Completer<()>,
}

unsafe extern "C" fn poll(user: *mut c_void) {
    let fence = unsafe { Box::from_raw(user.cast::<Fence>()) };
    let status = unsafe { glClientWaitSync(fence.sync, 0, 0) };
    if status == TIMEOUT_EXPIRED {
        unsafe { emscripten_set_timeout(poll, 1.0, Box::into_raw(fence).cast()) };
        return;
    }
    unsafe { glDeleteSync(fence.sync) };
    if status == WAIT_FAILED {
        fence
            .completer
            .reject(String::from("waiting on the WebGL fence failed"));
    } else {
        fence.completer.resolve(());
    }
}

impl Gl {
    pub fn open() -> Result<Gl, String> {
        let mut attributes = Attributes::default();
        unsafe { emscripten_webgl_init_context_attributes(&mut attributes) };
        attributes.major_version = 2;
        attributes.minor_version = 0;
        attributes.antialias = false;
        let context = unsafe { emscripten_webgl_create_context(CANVAS.as_ptr(), &attributes) };
        if context == 0 {
            return Err("no WebGL2 context on the web page's #canvas: this browser has no WebGL2, or the page no canvas".into());
        }
        if unsafe { emscripten_webgl_make_context_current(context) } != 0 {
            unsafe { emscripten_webgl_destroy_context(context) };
            return Err("making the WebGL2 context current failed".into());
        }
        Ok(Gl { context })
    }

    pub fn set_size(&self, width: u32, height: u32) {
        unsafe {
            emscripten_set_canvas_element_size(CANVAS.as_ptr(), width as c_int, height as c_int)
        };
    }

    pub fn drawing_buffer_size(&self) -> (i32, i32) {
        let (mut width, mut height) = (0, 0);
        unsafe { emscripten_webgl_get_drawing_buffer_size(self.context, &mut width, &mut height) };
        (width, height)
    }

    pub fn parameter(&self, name: u32) -> String {
        text(unsafe { glGetString(name) })
    }

    pub fn extensions(&self) -> Vec<String> {
        let mut count = 0;
        unsafe { glGetIntegerv(NUM_EXTENSIONS, &mut count) };
        (0..count.max(0) as u32)
            .map(|i| text(unsafe { glGetStringi(EXTENSIONS, i) }))
            .collect()
    }

    pub fn error(&self) -> u32 {
        unsafe { glGetError() }
    }

    pub fn create(&self, kind: Kind) -> Result<u32, String> {
        let mut name = 0;
        unsafe {
            match kind {
                Kind::Buffer => glGenBuffers(1, &mut name),
                Kind::Texture => glGenTextures(1, &mut name),
                Kind::Sampler => glGenSamplers(1, &mut name),
                Kind::VertexArray => glGenVertexArrays(1, &mut name),
                Kind::Framebuffer => glGenFramebuffers(1, &mut name),
                Kind::Program => name = glCreateProgram(),
                Kind::Shader => unreachable!("a shader is made with create_shader"),
            }
        }
        if name == 0 {
            return Err(format!(
                "WebGL could not create the object: error 0x{:X}",
                self.error()
            ));
        }
        Ok(name)
    }

    pub fn create_shader(&self, kind: u32) -> Result<u32, String> {
        match unsafe { glCreateShader(kind) } {
            0 => Err(format!(
                "WebGL could not create a shader: error 0x{:X}",
                self.error()
            )),
            name => Ok(name),
        }
    }

    pub fn delete(&self, kind: Kind, name: u32) {
        unsafe {
            match kind {
                Kind::Buffer => glDeleteBuffers(1, &name),
                Kind::Texture => glDeleteTextures(1, &name),
                Kind::Sampler => glDeleteSamplers(1, &name),
                Kind::VertexArray => glDeleteVertexArrays(1, &name),
                Kind::Framebuffer => glDeleteFramebuffers(1, &name),
                Kind::Program => glDeleteProgram(name),
                Kind::Shader => glDeleteShader(name),
            }
        }
    }

    pub fn bind_buffer(&self, target: u32, name: u32) {
        unsafe { glBindBuffer(target, name) };
    }

    pub fn bind_buffer_base(&self, target: u32, index: u32, name: u32) {
        unsafe { glBindBufferBase(target, index, name) };
    }

    pub fn buffer_data_size(&self, target: u32, size: i32, usage: u32) {
        unsafe { glBufferData(target, size as isize, std::ptr::null(), usage) };
    }

    pub fn buffer_data(&self, target: u32, bytes: &[u8], usage: u32) {
        unsafe { glBufferData(target, bytes.len() as isize, bytes.as_ptr().cast(), usage) };
    }

    pub fn buffer_sub_data(&self, target: u32, offset: i32, bytes: &[u8]) {
        unsafe {
            glBufferSubData(
                target,
                offset as isize,
                bytes.len() as isize,
                bytes.as_ptr().cast(),
            )
        };
    }

    pub fn get_buffer_sub_data(&self, target: u32, offset: i32, bytes: &mut [u8]) {
        unsafe {
            glGetBufferSubData(
                target,
                offset as isize,
                bytes.len() as isize,
                bytes.as_mut_ptr().cast(),
            )
        };
    }

    pub fn bind_texture(&self, target: u32, name: u32) {
        unsafe { glBindTexture(target, name) };
    }

    pub fn active_texture(&self, unit: u32) {
        unsafe { glActiveTexture(unit) };
    }

    pub fn tex_storage_2d(&self, format: u32, width: i32, height: i32) {
        unsafe { glTexStorage2D(gl::TEXTURE_2D, 1, format, width, height) };
    }

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
        unsafe {
            glTexSubImage2D(
                gl::TEXTURE_2D,
                0,
                x,
                y,
                width,
                height,
                format,
                kind,
                pixels.as_ptr().cast(),
            )
        };
        Ok(())
    }

    pub fn pixel_store(&self, name: u32, value: i32) {
        unsafe { glPixelStorei(name, value) };
    }

    pub fn sampler_parameter(&self, sampler: u32, name: u32, value: i32) {
        unsafe { glSamplerParameteri(sampler, name, value) };
    }

    pub fn bind_sampler(&self, unit: u32, sampler: u32) {
        unsafe { glBindSampler(unit, sampler) };
    }

    pub fn shader_source(&self, shader: u32, source: &str) {
        let pointer = source.as_ptr().cast::<c_char>();
        let length = source.len() as c_int;
        unsafe { glShaderSource(shader, 1, &pointer, &length) };
    }

    pub fn compile_shader(&self, shader: u32) {
        unsafe { glCompileShader(shader) };
    }

    pub fn compiled(&self, shader: u32) -> bool {
        let mut status = 0;
        unsafe { glGetShaderiv(shader, COMPILE_STATUS, &mut status) };
        status != 0
    }

    pub fn shader_log(&self, shader: u32) -> String {
        let mut length = 0;
        unsafe { glGetShaderiv(shader, INFO_LOG_LENGTH, &mut length) };
        log(length, |size, written, out| unsafe {
            glGetShaderInfoLog(shader, size, written, out)
        })
    }

    pub fn attach_shader(&self, program: u32, shader: u32) {
        unsafe { glAttachShader(program, shader) };
    }

    pub fn detach_shader(&self, program: u32, shader: u32) {
        unsafe { glDetachShader(program, shader) };
    }

    pub fn link_program(&self, program: u32) {
        unsafe { glLinkProgram(program) };
    }

    pub fn linked(&self, program: u32) -> bool {
        let mut status = 0;
        unsafe { glGetProgramiv(program, LINK_STATUS, &mut status) };
        status != 0
    }

    pub fn program_log(&self, program: u32) -> String {
        let mut length = 0;
        unsafe { glGetProgramiv(program, INFO_LOG_LENGTH, &mut length) };
        log(length, |size, written, out| unsafe {
            glGetProgramInfoLog(program, size, written, out)
        })
    }

    pub fn use_program(&self, program: u32) {
        unsafe { glUseProgram(program) };
    }

    pub fn uniform_block_index(&self, program: u32, name: &str) -> u32 {
        unsafe { glGetUniformBlockIndex(program, c_text(name).as_ptr()) }
    }

    pub fn uniform_block_binding(&self, program: u32, index: u32, binding: u32) {
        unsafe { glUniformBlockBinding(program, index, binding) };
    }

    pub fn sampler_unit(&self, program: u32, name: &str, unit: i32) -> bool {
        let location = unsafe { glGetUniformLocation(program, c_text(name).as_ptr()) };
        if location < 0 {
            return false;
        }
        unsafe {
            glUseProgram(program);
            glUniform1i(location, unit);
        }
        true
    }

    pub fn bind_vertex_array(&self, name: u32) {
        unsafe { glBindVertexArray(name) };
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
        unsafe {
            glVertexAttribPointer(
                location,
                size,
                kind,
                normalized as u8,
                stride,
                offset as usize as *const c_void,
            )
        };
    }

    pub fn vertex_attrib_i_pointer(
        &self,
        location: u32,
        size: i32,
        kind: u32,
        stride: i32,
        offset: i32,
    ) {
        unsafe {
            glVertexAttribIPointer(
                location,
                size,
                kind,
                stride,
                offset as usize as *const c_void,
            )
        };
    }

    pub fn enable_vertex_attrib_array(&self, location: u32) {
        unsafe { glEnableVertexAttribArray(location) };
    }

    pub fn vertex_attrib_divisor(&self, location: u32, divisor: u32) {
        unsafe { glVertexAttribDivisor(location, divisor) };
    }

    pub fn bind_framebuffer(&self, name: u32) {
        unsafe { glBindFramebuffer(gl::FRAMEBUFFER, name) };
    }

    pub fn framebuffer_texture_2d(&self, attachment: u32, texture: u32) {
        unsafe { glFramebufferTexture2D(gl::FRAMEBUFFER, attachment, gl::TEXTURE_2D, texture, 0) };
    }

    pub fn check_framebuffer_status(&self) -> u32 {
        unsafe { glCheckFramebufferStatus(gl::FRAMEBUFFER) }
    }

    pub fn viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { glViewport(x, y, width, height) };
    }

    pub fn scissor(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe { glScissor(x, y, width, height) };
    }

    pub fn clear(&self, bits: u32, [r, g, b, a]: [f32; 4]) {
        unsafe {
            glClearColor(r, g, b, a);
            glClear(bits);
        }
    }

    pub fn draw_arrays_instanced(&self, mode: u32, first: i32, count: i32, instances: i32) {
        unsafe { glDrawArraysInstanced(mode, first, count, instances) };
    }

    pub fn draw_elements_instanced(
        &self,
        mode: u32,
        count: i32,
        kind: u32,
        offset: i32,
        instances: i32,
    ) {
        unsafe {
            glDrawElementsInstanced(
                mode,
                count,
                kind,
                offset as usize as *const c_void,
                instances,
            )
        };
    }

    pub fn read_pixels(&self, x: i32, y: i32, width: i32, height: i32, pixels: &mut [u8]) {
        unsafe {
            glReadPixels(
                x,
                y,
                width,
                height,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                pixels.as_mut_ptr().cast(),
            )
        };
    }

    pub fn flush(&self) {
        unsafe { glFlush() };
    }

    pub fn finish(&self) {
        unsafe { glFinish() };
    }

    /// Settles when a fence put in now has signaled, polled from the page's event loop.
    pub fn completed(&self) -> Result<hxn_future::Future<()>, String> {
        let sync = unsafe { glFenceSync(SYNC_GPU_COMMANDS_COMPLETE, 0) };
        if sync.is_null() {
            return Err(format!(
                "creating a WebGL fence failed: error 0x{:X}",
                self.error()
            ));
        }
        unsafe { glFlush() };
        let (completer, future) = hxn_future::pending();
        let fence = Box::new(Fence { sync, completer });
        unsafe { emscripten_set_timeout(poll, 0.0, Box::into_raw(fence).cast()) };
        Ok(future)
    }
}
