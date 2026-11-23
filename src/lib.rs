//! WebGL2 for Haxe in the browser, keeping OpenGL ES 3.0's conventions as hxn-gpu-gles does: a
//! program makes a context on the web page's canvas, creates objects through it, and binds them
//! by unit and binding point. Shaders are hxnsl's GLSL ES 3.00, bound by the names hxnsl reports
//! in `GLSL_BINDINGS`. WebGL2 has no compute shaders or storage buffers.
//!
//! Two backends, by how the program reaches the web page: without threads, Emscripten's OpenGL
//! ES 3.0 functions over the page's WebGL2 context; with threads, under Ash or Emscripten, a
//! wire of commands in shared memory, served by a browser agent the web page starts through
//! `ash_host_agent`. This file is the API both share, and the one hxn generates the Haxe classes
//! from.

#![cfg(any(
    target_os = "emscripten",
    all(target_os = "wasi", target_feature = "atomics")
))]
// The browser agent backend's wire waits and notifies with wasm's atomics.
#![cfg_attr(target_feature = "atomics", feature(stdarch_wasm_atomic_wait))]

// Without threads, Emscripten's WebGL2 on the page's thread.
#[cfg(all(target_os = "emscripten", not(target_feature = "atomics")))]
#[path = "emscripten.rs"]
mod backend;

// With threads, under Ash or Emscripten, a browser agent in a Worker of its own, so the
// program's thread can wait for what the browser settles.
#[cfg(target_feature = "atomics")]
#[path = "agent/mod.rs"]
mod backend;

use std::sync::Arc;

use hxn_future::Future;

hxn::package!("hxn.gpu.webgl2");

/// The OpenGL ES values this library passes, which WebGL2 shares.
#[allow(dead_code)]
mod gl {
    pub const BLEND: u32 = 0x0BE2;
    pub const ZERO: u32 = 0;
    pub const ONE: u32 = 1;
    pub const SRC_COLOR: u32 = 0x0300;
    pub const ONE_MINUS_SRC_COLOR: u32 = 0x0301;
    pub const SRC_ALPHA: u32 = 0x0302;
    pub const ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
    pub const DST_ALPHA: u32 = 0x0304;
    pub const ONE_MINUS_DST_ALPHA: u32 = 0x0305;
    pub const DST_COLOR: u32 = 0x0306;
    pub const ONE_MINUS_DST_COLOR: u32 = 0x0307;
    pub const SRC_ALPHA_SATURATE: u32 = 0x0308;
    pub const FUNC_ADD: u32 = 0x8006;
    pub const MIN: u32 = 0x8007;
    pub const MAX: u32 = 0x8008;
    pub const FUNC_SUBTRACT: u32 = 0x800A;
    pub const FUNC_REVERSE_SUBTRACT: u32 = 0x800B;
    pub const VENDOR: u32 = 0x1F00;
    pub const RENDERER: u32 = 0x1F01;
    pub const VERSION: u32 = 0x1F02;
    pub const TEXTURE_2D: u32 = 0x0DE1;
    pub const TEXTURE0: u32 = 0x84C0;
    pub const TEXTURE_MAG_FILTER: u32 = 0x2800;
    pub const TEXTURE_MIN_FILTER: u32 = 0x2801;
    pub const TEXTURE_WRAP_S: u32 = 0x2802;
    pub const TEXTURE_WRAP_T: u32 = 0x2803;
    pub const FRAMEBUFFER: u32 = 0x8D40;
    pub const READ_FRAMEBUFFER: u32 = 0x8CA8;
    pub const DRAW_FRAMEBUFFER: u32 = 0x8CA9;
    pub const COLOR_BUFFER_BIT: u32 = 0x4000;
    pub const NEAREST: u32 = 0x2600;
    pub const SCISSOR_TEST: u32 = 0x0C11;
    pub const COLOR: u32 = 0x1800;
    pub const DEPTH: u32 = 0x1801;
    pub const DEPTH_TEST: u32 = 0x0B71;
    pub const NEVER: u32 = 0x0200;
    pub const FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;
    pub const COLOR_ATTACHMENT0: u32 = 0x8CE0;
    pub const DEPTH_ATTACHMENT: u32 = 0x8D00;
    pub const DEPTH_STENCIL_ATTACHMENT: u32 = 0x821A;
    pub const VERTEX_SHADER: u32 = 0x8B31;
    pub const FRAGMENT_SHADER: u32 = 0x8B30;
    pub const ARRAY_BUFFER: u32 = 0x8892;
    pub const ELEMENT_ARRAY_BUFFER: u32 = 0x8893;
    pub const UNIFORM_BUFFER: u32 = 0x8A11;
    pub const COPY_READ_BUFFER: u32 = 0x8F36;
    pub const PIXEL_UNPACK_BUFFER: u32 = 0x88EC;
    pub const STREAM_DRAW: u32 = 0x88E0;
    pub const PACK_ALIGNMENT: u32 = 0x0D05;
    pub const UNPACK_ALIGNMENT: u32 = 0x0CF5;
    pub const RED: u32 = 0x1903;
    pub const RG: u32 = 0x8227;
    pub const RGBA: u32 = 0x1908;
    pub const DEPTH_COMPONENT: u32 = 0x1902;
    pub const DEPTH_STENCIL: u32 = 0x84F9;
    pub const UNSIGNED_BYTE: u32 = 0x1401;
    pub const FLOAT: u32 = 0x1406;
    pub const HALF_FLOAT: u32 = 0x140B;
    pub const UNSIGNED_INT_24_8: u32 = 0x84FA;
    pub const INVALID_INDEX: u32 = 0xFFFF_FFFF;
}

fn u32_of(value: i32, what: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{what} cannot be {value}"))
}

fn i32_of(value: i64, what: &str) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("{what} cannot be {value}"))
}

/// `Blend` sets how what is drawn blends into the target. The factors are hxn-gpu's shared
/// codes, from 0 `Zero` to 10 `SrcAlphaSaturated`, and the operations are the codes from 0 `Add`
/// to 4 `Max`. When `enabled` is false, drawing writes its color as it is.
#[hxn::data]
pub struct Blend {
    pub enabled: bool,
    pub color_operation: i32,
    pub color_source: i32,
    pub color_destination: i32,
    pub alpha_operation: i32,
    pub alpha_source: i32,
    pub alpha_destination: i32,
}

/// A texture's internal format, OpenGL ES's values.
#[hxn::data]
#[derive(Clone, Copy, PartialEq)]
pub enum TextureFormat {
    R8 = 0x8229,
    Rg8 = 0x822B,
    Rgba8 = 0x8058,
    Srgb8Alpha8 = 0x8C43,
    R32f = 0x822E,
    Rgba16f = 0x881A,
    Rgba32f = 0x8814,
    Depth24Stencil8 = 0x88F0,
    Depth32f = 0x8CAC,
}

impl TextureFormat {
    /// The format and type `texSubImage2D` takes for it.
    fn transfer(self) -> (u32, u32) {
        match self {
            TextureFormat::R8 => (gl::RED, gl::UNSIGNED_BYTE),
            TextureFormat::Rg8 => (gl::RG, gl::UNSIGNED_BYTE),
            TextureFormat::Rgba8 | TextureFormat::Srgb8Alpha8 => (gl::RGBA, gl::UNSIGNED_BYTE),
            TextureFormat::R32f => (gl::RED, gl::FLOAT),
            TextureFormat::Rgba16f => (gl::RGBA, gl::HALF_FLOAT),
            TextureFormat::Rgba32f => (gl::RGBA, gl::FLOAT),
            TextureFormat::Depth24Stencil8 => (gl::DEPTH_STENCIL, gl::UNSIGNED_INT_24_8),
            TextureFormat::Depth32f => (gl::DEPTH_COMPONENT, gl::FLOAT),
        }
    }

    fn bytes_per_pixel(self) -> usize {
        match self {
            TextureFormat::R8 => 1,
            TextureFormat::Rg8 => 2,
            TextureFormat::Rgba16f => 8,
            TextureFormat::Rgba32f => 16,
            _ => 4,
        }
    }

    fn attachment(self) -> u32 {
        match self {
            TextureFormat::Depth24Stencil8 => gl::DEPTH_STENCIL_ATTACHMENT,
            TextureFormat::Depth32f => gl::DEPTH_ATTACHMENT,
            _ => gl::COLOR_ATTACHMENT0,
        }
    }
}

/// The binding target a buffer is created for, OpenGL ES's values.
#[hxn::data]
#[derive(Clone, Copy)]
pub enum BufferTarget {
    Array = 0x8892,
    ElementArray = 0x8893,
    Uniform = 0x8A11,
    CopyRead = 0x8F36,
    CopyWrite = 0x8F37,
    PixelPack = 0x88EB,
    PixelUnpack = 0x88EC,
}

/// A buffer's usage hint, OpenGL ES's values.
#[hxn::data]
#[derive(Clone, Copy)]
pub enum Usage {
    StaticDraw = 0x88E4,
    DynamicDraw = 0x88E8,
    StreamDraw = 0x88E0,
    StaticRead = 0x88E5,
    DynamicRead = 0x88E9,
    StreamRead = 0x88E1,
}

/// A sampler's filter, OpenGL ES's values.
#[hxn::data]
#[derive(Clone, Copy)]
pub enum Filter {
    Nearest = 0x2600,
    Linear = 0x2601,
}

/// A sampler's wrap mode, OpenGL ES's values.
#[hxn::data]
#[derive(Clone, Copy)]
pub enum Wrap {
    Repeat = 0x2901,
    ClampToEdge = 0x812F,
    MirroredRepeat = 0x8370,
}

/// A draw's primitive, OpenGL ES's values.
#[hxn::data]
#[derive(Clone, Copy)]
pub enum Primitive {
    Points = 0,
    Lines = 1,
    LineLoop = 2,
    LineStrip = 3,
    Triangles = 4,
    TriangleStrip = 5,
    TriangleFan = 6,
}

/// A vertex attribute's or index's component type, OpenGL ES's values.
#[hxn::data]
#[derive(Clone, Copy)]
pub enum ComponentType {
    Byte = 0x1400,
    UnsignedByte = 0x1401,
    Short = 0x1402,
    UnsignedShort = 0x1403,
    Int = 0x1404,
    UnsignedInt = 0x1405,
    Float = 0x1406,
    HalfFloat = 0x140B,
}

/// `Context.clear` bits, OpenGL ES's values, or-ed together.
#[hxn::data]
#[derive(Clone, Copy)]
pub enum ClearBits {
    Depth = 0x100,
    Stencil = 0x400,
    Color = 0x4000,
}

/// The kinds of object a backend creates and deletes.
#[derive(Clone, Copy)]
enum Kind {
    Buffer,
    Texture,
    Sampler,
    Program,
    Shader,
    VertexArray,
    Framebuffer,
}

/// An object of `kind`, deleted when the last reference to it goes.
struct Name {
    gl: Arc<backend::Gl>,
    kind: Kind,
    name: u32,
}

impl Name {
    fn new(gl: &Arc<backend::Gl>, kind: Kind) -> Result<Name, String> {
        Ok(Name {
            gl: gl.clone(),
            kind,
            name: gl.create(kind)?,
        })
    }
}

impl Drop for Name {
    fn drop(&mut self) {
        self.gl.delete(self.kind, self.name);
    }
}

/// A WebGL2 context on the web page's canvas.
#[hxn::interface]
pub struct Context {
    gl: Arc<backend::Gl>,
}

#[hxn::methods]
impl Context {
    /// A WebGL2 context on the web page's canvas. Throws when the browser has no WebGL2 or the
    /// page no canvas.
    pub fn create() -> Result<Context, String> {
        hxn::log::debug!(target: "hxn.gpu.webgl2", "opening a browser WebGL2 context");
        Ok(Context {
            gl: Arc::new(backend::Gl::open()?),
        })
    }

    /// Sets the size of the canvas's drawing buffer, in pixels.
    pub fn set_size(&self, width: i32, height: i32) -> Result<(), String> {
        self.gl
            .set_size(u32_of(width, "a width")?, u32_of(height, "a height")?);
        Ok(())
    }

    /// The width of the canvas's drawing buffer.
    pub fn width(&self) -> i32 {
        self.gl.drawing_buffer_size().0
    }

    /// The height of the canvas's drawing buffer.
    pub fn height(&self) -> i32 {
        self.gl.drawing_buffer_size().1
    }

    /// `GL_RENDERER`.
    pub fn renderer(&self) -> String {
        self.gl.parameter(gl::RENDERER)
    }

    /// `GL_VERSION`.
    pub fn version(&self) -> String {
        self.gl.parameter(gl::VERSION)
    }

    /// The extensions WebGL2 supports here, separated by spaces.
    pub fn extensions(&self) -> String {
        self.gl.extensions().join(" ")
    }

    /// `getError`: 0, or the first error since the last call.
    pub fn error(&self) -> i32 {
        self.gl.error() as i32
    }

    /// A buffer of `size` bytes, zeroed, for `target`.
    pub fn create_buffer(
        &self,
        target: BufferTarget,
        size: i64,
        usage: Usage,
    ) -> Result<Buffer, String> {
        let size = i32_of(size, "a buffer's size")?;
        let name = Name::new(&self.gl, Kind::Buffer)?;
        self.gl.bind_buffer(target as u32, name.name);
        self.gl.buffer_data_size(target as u32, size, usage as u32);
        self.gl.bind_buffer(target as u32, 0);
        Ok(Buffer {
            name,
            target: target as u32,
            size,
        })
    }

    /// A buffer holding `bytes`, for `target`.
    pub fn create_buffer_with(
        &self,
        target: BufferTarget,
        bytes: &[u8],
        usage: Usage,
    ) -> Result<Buffer, String> {
        let size = i32_of(bytes.len() as i64, "a buffer's size")?;
        let name = Name::new(&self.gl, Kind::Buffer)?;
        self.gl.bind_buffer(target as u32, name.name);
        self.gl.buffer_data(target as u32, bytes, usage as u32);
        self.gl.bind_buffer(target as u32, 0);
        Ok(Buffer {
            name,
            target: target as u32,
            size,
        })
    }

    /// A 2D texture of one level, with immutable storage.
    pub fn create_texture(
        &self,
        width: i32,
        height: i32,
        format: TextureFormat,
    ) -> Result<Texture, String> {
        let name = Name::new(&self.gl, Kind::Texture)?;
        self.gl.bind_texture(gl::TEXTURE_2D, name.name);
        self.gl.tex_storage_2d(format as u32, width, height);
        self.gl.bind_texture(gl::TEXTURE_2D, 0);
        Ok(Texture {
            name,
            width,
            height,
            format,
        })
    }

    /// A sampler with these filters and wrap modes.
    pub fn create_sampler(
        &self,
        min: Filter,
        mag: Filter,
        wrap_s: Wrap,
        wrap_t: Wrap,
    ) -> Result<Sampler, String> {
        let name = Name::new(&self.gl, Kind::Sampler)?;
        for (parameter, value) in [
            (gl::TEXTURE_MIN_FILTER, min as i32),
            (gl::TEXTURE_MAG_FILTER, mag as i32),
            (gl::TEXTURE_WRAP_S, wrap_s as i32),
            (gl::TEXTURE_WRAP_T, wrap_t as i32),
        ] {
            self.gl.sampler_parameter(name.name, parameter, value);
        }
        Ok(Sampler { name })
    }

    /// A program linked from GLSL ES 3.00 vertex and fragment sources. Compile and link errors
    /// are thrown with WebGL's log.
    pub fn create_program(&self, vertex: &str, fragment: &str) -> Result<Program, String> {
        let gl = &self.gl;
        let program = Name::new(gl, Kind::Program)?;
        let mut shaders = Vec::new();
        for (kind, source, stage) in [
            (gl::VERTEX_SHADER, vertex, "vertex"),
            (gl::FRAGMENT_SHADER, fragment, "fragment"),
        ] {
            let shader = Name {
                gl: gl.clone(),
                kind: Kind::Shader,
                name: gl.create_shader(kind)?,
            };
            gl.shader_source(shader.name, source);
            gl.compile_shader(shader.name);
            if !gl.compiled(shader.name) {
                return Err(format!(
                    "compiling the {stage} shader: {}",
                    gl.shader_log(shader.name)
                ));
            }
            gl.attach_shader(program.name, shader.name);
            shaders.push(shader);
        }
        gl.link_program(program.name);
        for shader in &shaders {
            gl.detach_shader(program.name, shader.name);
        }
        if !gl.linked(program.name) {
            return Err(format!(
                "linking the program: {}",
                gl.program_log(program.name)
            ));
        }
        Ok(Program { name: program })
    }

    /// An empty vertex array.
    pub fn create_vertex_array(&self) -> Result<VertexArray, String> {
        Ok(VertexArray {
            name: Name::new(&self.gl, Kind::VertexArray)?,
        })
    }

    /// A framebuffer rendering into `color`. Throws unless it is complete.
    pub fn create_framebuffer(&self, color: &Texture) -> Result<Framebuffer, String> {
        let name = Name::new(&self.gl, Kind::Framebuffer)?;
        self.gl.bind_framebuffer(name.name);
        self.gl
            .framebuffer_texture_2d(color.format.attachment(), color.name.name);
        let status = self.gl.check_framebuffer_status();
        self.gl.bind_framebuffer(0);
        if status != gl::FRAMEBUFFER_COMPLETE {
            return Err(format!(
                "the framebuffer is incomplete: status 0x{status:X}"
            ));
        }
        Ok(Framebuffer {
            name,
            colors: u32::from(color.format.attachment() == gl::COLOR_ATTACHMENT0),
        })
    }

    /// Attaches `texture` to `framebuffer`: as its depth attachment when its format is a depth
    /// format, and otherwise as its next color attachment, which drawing then writes as the
    /// next output of the fragment shader.
    pub fn attach(&self, framebuffer: &mut Framebuffer, texture: &Texture) -> Result<(), String> {
        let attachment = match texture.format.attachment() {
            gl::COLOR_ATTACHMENT0 => {
                let next = gl::COLOR_ATTACHMENT0 + framebuffer.colors;
                framebuffer.colors += 1;
                next
            }
            depth => depth,
        };
        self.gl.bind_framebuffer(framebuffer.name.name);
        self.gl
            .framebuffer_texture_2d(attachment, texture.name.name);
        let buffers: Vec<u32> = (0..framebuffer.colors)
            .map(|i| gl::COLOR_ATTACHMENT0 + i)
            .collect();
        self.gl.draw_buffers(&buffers);
        let status = self.gl.check_framebuffer_status();
        self.gl.bind_framebuffer(0);
        if status != gl::FRAMEBUFFER_COMPLETE {
            return Err(format!(
                "the framebuffer is incomplete: status 0x{status:X}"
            ));
        }
        Ok(())
    }

    /// Clears color attachment `index` of the bound framebuffer to `r`, `g`, `b`, `a`, with
    /// `clearBufferfv`.
    pub fn clear_color_buffer(&self, index: i32, r: f64, g: f64, b: f64, a: f64) {
        self.gl
            .clear_buffer_color(index, [r as f32, g as f32, b as f32, a as f32]);
    }

    /// Clears the depth attachment of the bound framebuffer to `value`. Depth writes are on
    /// after it.
    pub fn clear_depth_buffer(&self, value: f64) {
        self.gl.depth_mask(true);
        self.gl.clear_buffer_depth(value as f32);
    }

    /// Sets the depth test of what is drawn next. With `enabled`, the test passes when
    /// `compare` holds between a fragment's depth and the depth in the target, with the codes
    /// from 0 `Never` to 7 `Always` that hxn-gpu's drivers share, and with `write` a fragment
    /// that passes writes its depth. Without `enabled`, no depth is tested.
    pub fn set_depth(&self, enabled: bool, compare: i32, write: bool) -> Result<(), String> {
        if !(0..=7).contains(&compare) {
            return Err(format!("a compare function is 0 to 7, not {compare}"));
        }
        if enabled {
            self.gl.enable(gl::DEPTH_TEST);
            self.gl.depth_func(gl::NEVER + compare as u32);
            self.gl.depth_mask(write);
        } else {
            self.gl.disable(gl::DEPTH_TEST);
        }
        Ok(())
    }

    /// Renders into `framebuffer` from now on.
    pub fn bind_framebuffer(&self, framebuffer: &Framebuffer) {
        self.gl.bind_framebuffer(framebuffer.name.name);
    }

    /// Renders into the web page's canvas from now on.
    pub fn bind_default_framebuffer(&self) {
        self.gl.bind_framebuffer(0);
    }

    pub fn viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        self.gl.viewport(x, y, width, height);
    }

    pub fn scissor(&self, x: i32, y: i32, width: i32, height: i32) {
        self.gl.scissor(x, y, width, height);
    }

    /// Turns the scissor test on or off: `glEnable` or `glDisable` of `GL_SCISSOR_TEST`.
    /// While it is on, drawing, clearing and copies stay inside the `scissor` rectangle.
    pub fn scissor_test(&self, enabled: bool) {
        if enabled {
            self.gl.enable(gl::SCISSOR_TEST);
        } else {
            self.gl.disable(gl::SCISSOR_TEST);
        }
    }

    /// Copies the `width` by `height` color of `framebuffer` into the canvas with its rows
    /// reversed, then renders into the canvas. The scissor test must be off.
    pub fn blit_flipped(&self, framebuffer: &Framebuffer, width: i32, height: i32) {
        self.gl.blit_flipped(framebuffer.name.name, width, height);
    }

    /// Clears the bound framebuffer's color, depth or stencil: `bits` are `ClearBits` or-ed
    /// together.
    pub fn clear(&self, bits: i32, r: f64, g: f64, b: f64, a: f64) {
        self.gl
            .clear(bits as u32, [r as f32, g as f32, b as f32, a as f32]);
    }

    /// Sets how what is drawn next blends into the target. It calls `enable(BLEND)` with
    /// `blendEquationSeparate` and `blendFuncSeparate`, or `disable(BLEND)`.
    pub fn set_blend(&self, blend: Blend) -> Result<(), String> {
        if !blend.enabled {
            self.gl.disable(gl::BLEND);
            return Ok(());
        }
        let factor = |code: i32, what: &str| -> Result<u32, String> {
            const FACTORS: [u32; 11] = [
                gl::ZERO,
                gl::ONE,
                gl::SRC_COLOR,
                gl::ONE_MINUS_SRC_COLOR,
                gl::SRC_ALPHA,
                gl::ONE_MINUS_SRC_ALPHA,
                gl::DST_COLOR,
                gl::ONE_MINUS_DST_COLOR,
                gl::DST_ALPHA,
                gl::ONE_MINUS_DST_ALPHA,
                gl::SRC_ALPHA_SATURATE,
            ];
            usize::try_from(code)
                .ok()
                .and_then(|i| FACTORS.get(i).copied())
                .ok_or_else(|| format!("{what} is {code}, and a blend factor is 0 to 10"))
        };
        let equation = |code: i32, what: &str| -> Result<u32, String> {
            const EQUATIONS: [u32; 5] = [
                gl::FUNC_ADD,
                gl::FUNC_SUBTRACT,
                gl::FUNC_REVERSE_SUBTRACT,
                gl::MIN,
                gl::MAX,
            ];
            usize::try_from(code)
                .ok()
                .and_then(|i| EQUATIONS.get(i).copied())
                .ok_or_else(|| format!("{what} is {code}, and a blend operation is 0 to 4"))
        };
        let color = equation(blend.color_operation, "the color operation")?;
        let alpha = equation(blend.alpha_operation, "the alpha operation")?;
        let sources = [
            factor(blend.color_source, "the color source")?,
            factor(blend.color_destination, "the color destination")?,
            factor(blend.alpha_source, "the alpha source")?,
            factor(blend.alpha_destination, "the alpha destination")?,
        ];
        self.gl.enable(gl::BLEND);
        self.gl.blend_equation_separate(color, alpha);
        self.gl
            .blend_func_separate(sources[0], sources[1], sources[2], sources[3]);
        Ok(())
    }

    pub fn use_program(&self, program: &Program) {
        self.gl.use_program(program.name.name);
    }

    pub fn bind_vertex_array(&self, vertex_array: &VertexArray) {
        self.gl.bind_vertex_array(vertex_array.name.name);
    }

    /// Binds `buffer` to uniform buffer binding point `index`.
    pub fn bind_uniform_buffer(&self, index: i32, buffer: &Buffer) -> Result<(), String> {
        let index = u32_of(index, "a binding point")?;
        self.gl
            .bind_buffer_base(gl::UNIFORM_BUFFER, index, buffer.name.name);
        Ok(())
    }

    /// Binds `texture` and `sampler` to texture unit `unit`.
    pub fn bind_texture(
        &self,
        unit: i32,
        texture: &Texture,
        sampler: &Sampler,
    ) -> Result<(), String> {
        let unit = u32_of(unit, "a texture unit")?;
        self.gl.active_texture(gl::TEXTURE0 + unit);
        self.gl.bind_texture(gl::TEXTURE_2D, texture.name.name);
        self.gl.bind_sampler(unit, sampler.name.name);
        Ok(())
    }

    /// `drawArraysInstanced`.
    pub fn draw_arrays(&self, primitive: Primitive, first: i32, count: i32, instances: i32) {
        self.gl
            .draw_arrays_instanced(primitive as u32, first, count, instances);
    }

    /// `drawElementsInstanced`, with indices from the bound vertex array's element buffer,
    /// starting `offset` bytes in.
    pub fn draw_elements(
        &self,
        primitive: Primitive,
        count: i32,
        kind: ComponentType,
        offset: i32,
        instances: i32,
    ) {
        self.gl
            .draw_elements_instanced(primitive as u32, count, kind as u32, offset, instances);
    }

    /// The bound framebuffer's pixels in a rectangle, as 8-bit RGBA, bottom row first. On the
    /// canvas, read in the same frame as the drawing: the browser clears the canvas after
    /// showing it.
    pub fn read_pixels(&self, x: i32, y: i32, width: i32, height: i32) -> Result<Vec<u8>, String> {
        let length = usize::try_from(width)
            .ok()
            .zip(usize::try_from(height).ok())
            .map(|(w, h)| w * h * 4)
            .ok_or("a rectangle's size cannot be negative")?;
        let mut pixels = vec![0u8; length];
        self.gl.pixel_store(gl::PACK_ALIGNMENT, 1);
        self.gl.read_pixels(x, y, width, height, &mut pixels);
        Ok(pixels)
    }

    /// `flush`.
    pub fn flush(&self) {
        self.gl.flush();
    }

    /// `finish`.
    pub fn finish(&self) {
        self.gl.finish();
    }

    /// Takes the web page's canvas, which `present` shows the drawing buffer on. With threads
    /// the context draws on a canvas of its own, and fails here when the page has no canvas
    /// or another browser agent holds it; without, it settles at once.
    pub fn claim_canvas(&self) -> Future<()> {
        self.gl.claim_canvas()
    }

    /// Shows what was drawn into the default framebuffer on the web page's canvas. Without
    /// threads the browser shows it once the program returns to its event loop.
    pub fn present(&self) {
        self.gl.present();
    }

    /// Settles when the GPU has finished every command so far, through a WebGL fence.
    pub fn completed(&self) -> Result<Future<()>, String> {
        self.gl.completed()
    }
}

/// A buffer object.
#[hxn::interface]
pub struct Buffer {
    name: Name,
    target: u32,
    size: i32,
}

#[hxn::methods]
impl Buffer {
    pub fn size(&self) -> i64 {
        self.size as i64
    }

    /// Writes `bytes` at `offset`.
    pub fn write(&self, offset: i64, bytes: &[u8]) -> Result<(), String> {
        let offset = i32_of(offset, "an offset")?;
        if offset < 0 || offset as i64 + bytes.len() as i64 > self.size as i64 {
            return Err(format!(
                "{} bytes at {offset} overrun a buffer of {}",
                bytes.len(),
                self.size
            ));
        }
        let gl = &self.name.gl;
        gl.bind_buffer(self.target, self.name.name);
        gl.buffer_sub_data(self.target, offset, bytes);
        gl.bind_buffer(self.target, 0);
        Ok(())
    }

    /// Reads `length` bytes at `offset`, with `getBufferSubData`. It waits for the GPU to finish
    /// writing them.
    pub fn read(&self, offset: i64, length: i64) -> Result<Vec<u8>, String> {
        let offset = i32_of(offset, "an offset")?;
        let length = i32_of(length, "a length")?;
        if offset < 0 || length < 0 || offset as i64 + length as i64 > self.size as i64 {
            return Err(format!(
                "{length} bytes at {offset} overrun a buffer of {}",
                self.size
            ));
        }
        let gl = &self.name.gl;
        let mut bytes = vec![0u8; length as usize];
        gl.bind_buffer(gl::COPY_READ_BUFFER, self.name.name);
        gl.get_buffer_sub_data(gl::COPY_READ_BUFFER, offset, &mut bytes);
        gl.bind_buffer(gl::COPY_READ_BUFFER, 0);
        Ok(bytes)
    }
}

/// A 2D texture of one level.
#[hxn::interface]
pub struct Texture {
    name: Name,
    width: i32,
    height: i32,
    format: TextureFormat,
}

#[hxn::methods]
impl Texture {
    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// Writes a rectangle of pixels, rows tightly packed, bottom row first.
    pub fn write(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        pixels: &[u8],
    ) -> Result<(), String> {
        let expected = usize::try_from(width)
            .ok()
            .zip(usize::try_from(height).ok())
            .map(|(w, h)| w * h * self.format.bytes_per_pixel())
            .ok_or("a rectangle's size cannot be negative")?;
        if pixels.len() != expected {
            return Err(format!(
                "{width}x{height} pixels are {expected} bytes, not {}",
                pixels.len()
            ));
        }
        let (format, kind) = self.format.transfer();
        let gl = &self.name.gl;
        gl.bind_texture(gl::TEXTURE_2D, self.name.name);
        gl.pixel_store(gl::UNPACK_ALIGNMENT, 1);
        gl.tex_sub_image_2d(x, y, width, height, format, kind, pixels)?;
        gl.bind_texture(gl::TEXTURE_2D, 0);
        Ok(())
    }
}

/// A sampler object.
#[hxn::interface]
pub struct Sampler {
    name: Name,
}

#[hxn::methods]
impl Sampler {}

/// A linked program.
#[hxn::interface]
pub struct Program {
    name: Name,
}

#[hxn::methods]
impl Program {
    /// Assigns the uniform block `name` to binding point `binding`. False when the program has no
    /// such block.
    pub fn uniform_block(&self, name: &str, binding: i32) -> Result<bool, String> {
        let binding = u32_of(binding, "a binding point")?;
        let gl = &self.name.gl;
        let index = gl.uniform_block_index(self.name.name, name);
        if index == gl::INVALID_INDEX {
            return Ok(false);
        }
        gl.uniform_block_binding(self.name.name, index, binding);
        Ok(true)
    }

    /// Assigns the sampler uniform `name` to texture unit `unit`. False when the program has no
    /// such uniform. It leaves this program in use.
    pub fn sampler(&self, name: &str, unit: i32) -> bool {
        self.name.gl.sampler_unit(self.name.name, name, unit)
    }
}

/// A vertex array: attribute layouts and the buffers they read, and an element buffer.
#[hxn::interface]
pub struct VertexArray {
    name: Name,
}

#[hxn::methods]
impl VertexArray {
    /// Attribute `location` reads `size` components of `kind` from `buffer`, `stride` bytes
    /// apart from `offset`, as floats; integer components become floats, scaled to 0-1 or -1-1
    /// when `normalized`.
    #[allow(clippy::too_many_arguments)]
    pub fn attribute(
        &self,
        location: i32,
        buffer: &Buffer,
        size: i32,
        kind: ComponentType,
        normalized: bool,
        stride: i32,
        offset: i32,
    ) -> Result<(), String> {
        let location = u32_of(location, "a location")?;
        let gl = &self.name.gl;
        gl.bind_vertex_array(self.name.name);
        gl.bind_buffer(gl::ARRAY_BUFFER, buffer.name.name);
        gl.vertex_attrib_pointer(location, size, kind as u32, normalized, stride, offset);
        gl.enable_vertex_attrib_array(location);
        gl.bind_buffer(gl::ARRAY_BUFFER, 0);
        gl.bind_vertex_array(0);
        Ok(())
    }

    /// Attribute `location` reads `size` integer components of `kind`, for an `int` or `uint`
    /// input.
    pub fn integer_attribute(
        &self,
        location: i32,
        buffer: &Buffer,
        size: i32,
        kind: ComponentType,
        stride: i32,
        offset: i32,
    ) -> Result<(), String> {
        let location = u32_of(location, "a location")?;
        let gl = &self.name.gl;
        gl.bind_vertex_array(self.name.name);
        gl.bind_buffer(gl::ARRAY_BUFFER, buffer.name.name);
        gl.vertex_attrib_i_pointer(location, size, kind as u32, stride, offset);
        gl.enable_vertex_attrib_array(location);
        gl.bind_buffer(gl::ARRAY_BUFFER, 0);
        gl.bind_vertex_array(0);
        Ok(())
    }

    /// Attribute `location` advances once per `divisor` instances; 0 advances per vertex.
    pub fn divisor(&self, location: i32, divisor: i32) -> Result<(), String> {
        let location = u32_of(location, "a location")?;
        let divisor = u32_of(divisor, "a divisor")?;
        let gl = &self.name.gl;
        gl.bind_vertex_array(self.name.name);
        gl.vertex_attrib_divisor(location, divisor);
        gl.bind_vertex_array(0);
        Ok(())
    }

    /// The buffer `drawElements` reads indices from.
    pub fn element_buffer(&self, buffer: &Buffer) {
        let gl = &self.name.gl;
        gl.bind_vertex_array(self.name.name);
        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, buffer.name.name);
        gl.bind_vertex_array(0);
    }
}

/// A framebuffer with one attachment.
#[hxn::interface]
pub struct Framebuffer {
    name: Name,
    // The number of color attachments, which `drawBuffers` draws into.
    colors: u32,
}

#[hxn::methods]
impl Framebuffer {}
