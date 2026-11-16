package hxn.gpu.webgl2;

/** Shaders from hxnsl, for WebGL2. **/
class Shaders {
	#if macro
	/**
		Registers hxnsl's GLSL target for `hxn.gpu.webgl2.Shader`. The package's
		extraParams.hxml runs it; without hxnsl it does nothing.
	**/
	public static function register():Void {
		#if hxnsl
		hxnsl.Targets.register(new hxnsl.targets.Glsl(), "hxn.gpu.webgl2.Shader");
		#end
	}
	#elseif hxnsl

	/**
		A program linked from a shader's GLSL, with `bindings` assigned to it.
		`vertex` and `fragment` are its `GLSL_VERTEX` and `GLSL_FRAGMENT`.
	**/
	public static function program(context:Context, vertex:String, fragment:String, bindings:Bindings):Program {
		var program = context.createProgram(vertex, fragment);
		bindings.apply(program);
		return program;
	}
	#end
}
