# hxn-gpu-webgl2

[![Browser](https://github.com/hx-n/hxn-gpu-webgl2/actions/workflows/browser.yml/badge.svg)](https://github.com/hx-n/hxn-gpu-webgl2/actions/workflows/browser.yml)

WebGL2 bindings for Haxe programs that run in the browser, for browsers and
devices without WebGPU. The classes are the ones
[hxn-gpu-gles](https://github.com/hx-n/hxn-gpu-gles) has for OpenGL ES 3.0:
`Context`, `Buffer`, `Texture`, `Sampler`, `Program`, `VertexArray` and
`Framebuffer`. Textures and uniform buffers are bound to numbered texture
units and binding points, as in OpenGL ES. `Context.create()` makes the
context on the web page's canvas. The library is written in Rust.

| Runtime | Browser support |
|---|---|
| hxcpp | Supported, with or without threads. Compile with `-cpp -D emscripten`, and add `-D hxn_threads` for threads. |
| [Ash](https://github.com/rayzor-blade/ash) | Supported in threaded builds. Compile with `-hl -D wasm -D hxn_threads`. |
| HashLink | Not supported: the HashLink VM does not run in a browser. Use Ash. |
| CPPIA | Not supported: the script host is a native hxcpp program. |

Where the browser has WebGPU, use
[hxn-gpu-webgpu](https://github.com/hx-n/hxn-gpu-webgpu). Outside the browser,
use the native drivers:
[hxn-gpu-metal](https://github.com/hx-n/hxn-gpu-metal),
[hxn-gpu-vulkan](https://github.com/hx-n/hxn-gpu-vulkan),
[hxn-gpu-d3d12](https://github.com/hx-n/hxn-gpu-d3d12) and
[hxn-gpu-gles](https://github.com/hx-n/hxn-gpu-gles).

## Shaders

Shaders are [hxnsl](https://github.com/hx-n/hxnsl) classes that implement
`hxn.gpu.webgl2.Shader`, which extends `hxnsl.Shader`. The `SRC` block is
written in HXSL. At compile time hxnsl adds static fields to the class, which
hxnsl's README lists in full:

- `GLSL_VERTEX` and `GLSL_FRAGMENT`, the GLSL ES 3.00 sources;
- `GLSL_BINDINGS`, the uniform blocks and textures each stage uses;
- constants for its inputs and bindings, such as `INPUT_position`,
  `TEXTURE_image` and `TEXTURE_image_GROUP`.

```haxe
class Textured implements hxn.gpu.webgl2.Shader {
	static var SRC = {
		@input var input : { position : Vec2, uv : Vec2 };
		var output : { position : Vec4, color : Vec4 };
		@param var image : Sampler2D;
		var calculatedUV : Vec2;

		function vertex() {
			calculatedUV = input.uv;
			output.position = vec4(input.position, 0, 1);
		}

		function fragment() {
			output.color = image.get(calculatedUV);
		}
	};
}
```

WebGL2 has no compute shaders and no storage buffers. A shader that uses them
gets a `GLSL_MISSING` field, a string saying why, and no GLSL fields.

## Drawing

```haxe
import hxn.gpu.webgl2.*;

var context = Context.create();                   // throws without WebGL2 or a canvas
context.setSize(640, 480);                        // the canvas's drawing buffer, in pixels
var bindings = new Bindings(Textured.GLSL_BINDINGS);
var program = Shaders.program(context, Textured.GLSL_VERTEX, Textured.GLSL_FRAGMENT, bindings);

var vertices = context.createVertexArray();
var corners = context.createBufferWith(BufferTarget.Array, bytes, Usage.StaticDraw);
vertices.attribute(Textured.INPUT_position, corners, 2, ComponentType.Float, false, 16, 0);
vertices.attribute(Textured.INPUT_uv, corners, 2, ComponentType.Float, false, 16, 8);
var image = context.createTexture(w, h, TextureFormat.Rgba8);
image.write(0, 0, w, h, pixels);                  // rows tightly packed, bottom row first
var sampler = context.createSampler(Filter.Linear, Filter.Linear, Wrap.ClampToEdge, Wrap.ClampToEdge);

context.bindDefaultFramebuffer();                 // the canvas
context.viewport(0, 0, 640, 480);
context.clear(ClearBits.Color, 0, 0, 0, 1);
context.useProgram(program);
context.bindVertexArray(vertices);
context.bindTexture(bindings.textureUnit(Textured.TEXTURE_image_GROUP, Textured.TEXTURE_image), image, sampler);
context.drawArrays(Primitive.TriangleStrip, 0, 4, 1);
```

`Shaders.program` links the program and assigns its uniform blocks and
samplers the binding points and texture units that `Bindings` numbers.

A texture `@param` is bound with `bindTexture`, as above. The shader's other
`@param` values are read from a uniform buffer of `PARAMS_SIZE` bytes, each
value at its `PARAMS_<name>` offset:

```haxe
class Tinted implements hxn.gpu.webgl2.Shader {
	static var SRC = {
		@input var input : { position : Vec2 };
		var output : { position : Vec4, color : Vec4 };
		@param var tint : Vec4;

		function vertex() {
			output.position = vec4(input.position, 0, 1);
		}

		function fragment() {
			output.color = tint;
		}
	};
}

var params = haxe.io.Bytes.alloc(Tinted.PARAMS_SIZE);
params.setFloat(Tinted.PARAMS_tint, 0.25);        // and the other three components
var uniforms = context.createBufferWith(BufferTarget.Uniform, params, Usage.StaticDraw);
context.bindUniformBuffer(bindings.uniformBlock(Tinted.PARAMS_GROUP, Tinted.PARAMS_BINDING), uniforms);
```

The browser shows what was drawn on the canvas once the code that drew it
returns to the browser, and clears the canvas after showing it. To read the
canvas back, call `readPixels` before returning.

[`examples/check/Main.hx`](https://github.com/hx-n/hxn-gpu-webgl2/blob/main/examples/check/Main.hx)
renders into a texture and reads it back once the GPU has finished, samples a
texture, writes and reads a buffer, and clears the web page's canvas and
reads it back.

## Compiling

For hxcpp:

```
haxelib install hxn-gpu-webgl2
haxelib install hxnsl
haxe -lib hxn-gpu-webgl2 -lib hxnsl -main Main -cpp out -D emscripten -D disable_unicode_strings -D HXCPP_LINK_EMSCRIPTEN_EXT=.html
```

Requires the [Emscripten SDK](https://emscripten.org/docs/getting_started/downloads.html)
on `PATH` and hxcpp from git (its Emscripten toolchain uses wasm exceptions).
The output is `out/Main.html` with `Main.js` and `Main.wasm`; serve the
directory and open `Main.html` in a browser. The haxelib ships the library
prebuilt for `wasm32-unknown-emscripten` and links it with WebGL2
(`-sMIN_WEBGL_VERSION=2 -sMAX_WEBGL_VERSION=2`), so no Rust toolchain is
needed. The context is made on the element with the id `canvas`. Emscripten's
default HTML page has one; a page of your own needs a `<canvas id="canvas">`.

With `-D hxn_threads`, the program links the library built for threaded wasm
and runs on Emscripten's pthreads. Its `main` runs on a pthread, and the
canvas is transferred to that thread, which holds the context. A threaded
program uses shared memory, so the web page must be served with cross-origin
isolation headers (COOP/COEP), which `emrun` sends:

```
haxe -lib hxn-gpu-webgl2 -lib hxnsl -main Main -cpp out -D emscripten -D hxn_threads -D disable_unicode_strings -D HXCPP_LINK_EMSCRIPTEN_EXT=.html
emrun out/Main.html
```

For Ash 0.1.19 or later, the program is compiled with `-D wasm -D
hxn_threads`, which selects the library built for threaded wasm. Compiling
copies the library, `libgpu_webgl2.a`, and its JavaScript files next to
`main.hl`. `ash --build` links the library into `main.wasm` and writes a web
page next to it, and `ash serve` serves the directory:

```
haxe -lib hxn-gpu-webgl2 -lib hxnsl -main Main -hl out/main.hl -D wasm -D hxn_threads
ash --build out/main.wasm --target wasm32-wasip1-threads --native-lib out/libgpu_webgl2.a out/main.hl
ash serve out
```

A threaded program uses shared memory, so the web page must be served with
cross-origin isolation headers (COOP/COEP), which `ash serve` sends. Under
Ash, the program runs in a Worker, and the WebGL2 context is held by a browser
agent: a second Worker that the web page Ash writes starts with the library's
JavaScript files, and hands the page's canvas. A call that returns a value
(`readPixels`, `Buffer.read`, `error`, and the compile and link checks in
`Shaders.program`) waits for the browser agent to run every call made before
it, so make those calls after drawing rather than between draws.

## Classes

All in the package `hxn.gpu.webgl2`:

| Class | |
|---|---|
| `Context` | `create()` on the web page's canvas, `setSize`, `width` and `height`; object creation, framebuffer and program binding, draws, `readPixels`, `finish` and `completed`. |
| `Buffer` | `write`, and `read` with `getBufferSubData`. |
| `Texture` | A 2D texture of one level with immutable storage, `write` by rectangle. |
| `Sampler` | Filters and wrap modes, bound to a texture unit with a texture. |
| `Program` | A linked program; `uniformBlock(name, binding)` and `sampler(name, unit)`, which `Shaders.program` calls for an hxnsl shader. |
| `VertexArray` | `attribute`, `integerAttribute`, `divisor` and `elementBuffer`. |
| `Framebuffer` | A framebuffer rendering into one texture. |
| `Bindings` | The binding points and texture units of an hxnsl shader. |
| `Shaders` | `program(context, vertex, fragment, bindings)`: a program linked from an hxnsl shader's GLSL, with its bindings assigned. |

Every object holds its WebGL object until `close()`. The context stays alive
until it and everything made from it are closed. A shader that fails to
compile or link throws a Haxe exception with WebGL's log; `Context.error()`
is `getError`.

`Context.completed()` returns an `hxn.Future`, from the hxn haxelib this one
depends on. It settles once the GPU has finished every command issued before
the call. Under Ash, `await` works: the context is held by the browser agent,
not by the program's thread. Under hxcpp, with or without threads, use `then`
rather than `await`: the thread that holds the context checks for the GPU to
finish only while it is not blocked.

## Building from source

Requires the [hxn](https://github.com/hx-n/hxn) CLI, Rust nightly with
`rust-src` and the `wasm32-unknown-emscripten` and `wasm32-wasip1-threads`
targets, Haxe 4.3, hxcpp from git, hxnsl, the Emscripten SDK, Ash, node 22
and Chrome. `hxn check` builds the library and `examples/check` as a web
page, opens it in headless Chrome and prints what the program prints:

```
hxn check cpp --wasm --browser
hxn check cpp --wasm --threads --browser
hxn check hl --wasm --threads --browser
```

CI runs each on macOS with the runner's GPU and on Linux with SwiftShader.

Under Ash, the library encodes each WebGL call into the program's memory
(`src/agent/wire.rs`), and the browser agent decodes and runs it
(`page/gpu_webgl2_service.mjs`). Both files are generated from the WebGL IDL
in `tools/wire/idl/`. After changing the IDL, regenerate them with:

```
cargo run --manifest-path tools/wire/Cargo.toml
```

## Agent skill

[`skills/hxn-gpu-webgl2/SKILL.md`](https://github.com/hx-n/hxn-gpu-webgl2/blob/main/skills/hxn-gpu-webgl2/SKILL.md)
describes the library for coding agents. The haxelib includes it.

## License

Apache-2.0. `tools/wire/idl/webgl.idl` and `webgl2.idl` are The Khronos
Group's, under the MIT-style license in their headers.
