import hxn.gpu.webgl2.*;
import haxe.io.Bytes;
import Shaders;

/**
	What CI runs in a web page: a render pass into a texture, read back once the
	GPU has finished; a pass sampling a texture; a buffer read back; and a frame
	of the web page's canvas. It needs a browser with WebGL2, and prints the
	same lines in each.
**/
class Main {
	static function check(ok:Bool, message:String) {
		if (!ok)
			throw message;
	}

	static function closeAll(objects:Array<{function close():Void;}>) {
		for (o in objects)
			o.close();
	}

	static function main() {
		var context = try Context.create() catch (e:haxe.Exception) {
			Sys.println('no WebGL2: ${e.message}');
			return;
		}
		// The render pass completes in a callback from the event loop, after main returns,
		// and the other passes run from there.
		render(context, () -> {
			textured(context);
			buffer(context);
			canvas(context);
			context.close();
		});
		Sys.println("render: submitted");
	}

	/** A 4x4 Rgba8 texture and a framebuffer rendering into it, bound and cleared. **/
	static function target(context:Context):{texture:Texture, framebuffer:Framebuffer} {
		var texture = context.createTexture(4, 4, TextureFormat.Rgba8);
		var framebuffer = context.createFramebuffer(texture);
		context.bindFramebuffer(framebuffer);
		context.viewport(0, 0, 4, 4);
		context.clear(ClearBits.Color, 0, 0, 0, 0);
		return {texture: texture, framebuffer: framebuffer};
	}

	/** Reads the bound framebuffer's `size` square, and checks every pixel is `expected`. **/
	static function readTarget(context:Context, size:Int, expected:Array<Int>, label:String) {
		var pixels = context.readPixels(0, 0, size, size);
		for (i in 0...size * size)
			for (c in 0...4)
				check(pixels.get(i * 4 + c) == expected[c], '$label: pixel $i channel $c is ${pixels.get(i * 4 + c)}, not ${expected[c]}');
	}

	static function quad(context:Context, values:Array<Float>):Buffer {
		var bytes = Bytes.alloc(values.length * 4);
		for (i in 0...values.length)
			bytes.setFloat(i * 4, values[i]);
		return context.createBufferWith(BufferTarget.Array, bytes, Usage.StaticDraw);
	}

	static function render(context:Context, next:() -> Void) {
		var bindings = new Bindings(Tinted.GLSL_BINDINGS);
		var program = hxn.gpu.webgl2.Shaders.program(context, Tinted.GLSL_VERTEX, Tinted.GLSL_FRAGMENT, bindings);
		var positions = quad(context, [-1, -1, 1, -1, -1, 1, 1, 1]);
		var vertices = context.createVertexArray();
		vertices.attribute(Tinted.INPUT_position, positions, 2, ComponentType.Float, false, 8, 0);
		var params = Bytes.alloc(Tinted.PARAMS_SIZE);
		var tint = [0.25, 0.5, 0.75, 1.0];
		for (i in 0...4)
			params.setFloat(Tinted.PARAMS_tint + i * 4, tint[i]);
		var uniforms = context.createBufferWith(BufferTarget.Uniform, params, Usage.StaticDraw);

		var t = target(context);
		context.useProgram(program);
		context.bindVertexArray(vertices);
		context.bindUniformBuffer(bindings.uniformBlock(Tinted.PARAMS_GROUP, Tinted.PARAMS_BINDING), uniforms);
		context.drawArrays(Primitive.TriangleStrip, 0, 4, 1);
		context.completed().then(_ -> {
			var expected = [for (c in tint) Math.round(c * 255)];
			readTarget(context, 4, expected, "render");
			Sys.println('render: 4x4 pixels of ${expected.join(", ")}');
			closeAll([t.framebuffer, t.texture, uniforms, vertices, positions, program]);
			next();
		}, e -> {
			Sys.println('render failed: ${e.message}');
			hxn.Loop.exit(1);
		});
	}

	static function textured(context:Context) {
		var bindings = new Bindings(Textured.GLSL_BINDINGS);
		var program = hxn.gpu.webgl2.Shaders.program(context, Textured.GLSL_VERTEX, Textured.GLSL_FRAGMENT, bindings);
		var corners = quad(context, [-1, -1, 0, 1, 1, -1, 1, 1, -1, 1, 0, 0, 1, 1, 1, 0]);
		var vertices = context.createVertexArray();
		vertices.attribute(Textured.INPUT_position, corners, 2, ComponentType.Float, false, 16, 0);
		vertices.attribute(Textured.INPUT_uv, corners, 2, ComponentType.Float, false, 16, 8);

		// A 2x2 image of one color.
		var color = [40, 80, 120, 255];
		var image = context.createTexture(2, 2, TextureFormat.Rgba8);
		var pixels = Bytes.alloc(2 * 2 * 4);
		for (i in 0...4)
			for (c in 0...4)
				pixels.set(i * 4 + c, color[c]);
		image.write(0, 0, 2, 2, pixels);
		var sampler = context.createSampler(Filter.Nearest, Filter.Nearest, Wrap.ClampToEdge, Wrap.ClampToEdge);

		var t = target(context);
		context.useProgram(program);
		context.bindVertexArray(vertices);
		context.bindTexture(bindings.textureUnit(Textured.TEXTURE_image_GROUP, Textured.TEXTURE_image), image, sampler);
		context.drawArrays(Primitive.TriangleStrip, 0, 4, 1);
		readTarget(context, 4, color, "textured");
		Sys.println('textured: 4x4 pixels of ${color.join(", ")}');
		closeAll([t.framebuffer, t.texture, sampler, image, vertices, corners, program]);
	}

	static function buffer(context:Context) {
		var count = 16;
		var values = Bytes.alloc(count * 4);
		for (i in 0...count)
			values.setInt32(i * 4, i * 7);
		var data = context.createBuffer(BufferTarget.CopyRead, count * 4, Usage.StaticRead);
		data.write(0, values);
		var read = data.read(0, count * 4);
		for (i in 0...count)
			check(read.getInt32(i * 4) == i * 7, 'value $i is ${read.getInt32(i * 4)}, not ${i * 7}');
		Sys.println('buffer: $count values written and read back, the last ${read.getInt32((count - 1) * 4)}');
		data.close();
	}

	/** A frame of the web page's canvas cleared to red, read before the browser shows it. **/
	static function canvas(context:Context) {
		context.setSize(64, 64);
		context.bindDefaultFramebuffer();
		context.viewport(0, 0, 64, 64);
		context.clear(ClearBits.Color, 1, 0, 0, 1);
		readTarget(context, 64, [255, 0, 0, 255], "canvas");
		Sys.println('canvas: a ${context.width()}x${context.height()} frame cleared to red');
		var error = context.error();
		check(error == 0, 'WebGL error 0x${StringTools.hex(error)}');
	}
}
