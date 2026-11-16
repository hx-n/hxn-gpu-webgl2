package hxn.gpu.webgl2;

#if hxnsl
/**
	An hxnsl shader for WebGL2. It gets hxnsl's `GLSL_VERTEX`,
	`GLSL_FRAGMENT` and `GLSL_BINDINGS`, or `GLSL_MISSING` for a shader
	WebGL2 cannot run; `Bindings` assigns its binding points and
	texture units.
**/
interface Shader extends hxnsl.Shader {}
#end
