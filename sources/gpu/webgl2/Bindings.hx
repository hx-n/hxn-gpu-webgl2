package gpu.webgl2;

#if (!macro && hxnsl)
import hxnsl.Layout.GlslBindings;

/**
	The uniform buffer binding points and texture units of an hxnsl shader's
	GLSL, from its `GLSL_BINDINGS`: a binding point for each uniform block's
	group and binding, and a texture unit for each texture and the sampler it
	is sampled with, numbered from 0 in the order the stages list them.
	`apply` assigns them to a program by the names hxnsl gives each stage.
**/
class Bindings {
	/** The binding points used, 0 to `uniformBlocks - 1`. **/
	public var uniformBlocks(default, null) = 0;

	/** The texture units used, 0 to `textureUnits - 1`. **/
	public var textureUnits(default, null) = 0;

	final stages:Array<GlslBindings>;
	final blocks = new Map<String, Int>();
	final units = new Map<String, Int>();

	public function new(stages:Array<GlslBindings>) {
		this.stages = stages;
		for (stage in stages) {
			for (block in stage.uniformBlocks) {
				var key = '${block.group}:${block.binding}';
				if (!blocks.exists(key))
					blocks[key] = uniformBlocks++;
			}
			for (texture in stage.textures) {
				var key = unitKey(texture.group, texture.binding, texture.sampler);
				if (!units.exists(key))
					units[key] = textureUnits++;
			}
		}
	}

	/** Assigns every uniform block and sampler uniform of `program`. **/
	public function apply(program:Program):Void {
		for (stage in stages) {
			for (block in stage.uniformBlocks)
				program.uniformBlock(block.name, uniformBlock(block.group, block.binding));
			for (texture in stage.textures)
				program.sampler(texture.name, units[unitKey(texture.group, texture.binding, texture.sampler)]);
		}
	}

	/** The binding point of the uniform block at `group` and `binding`. **/
	public function uniformBlock(group:Int, binding:Int):Int {
		var point = blocks['$group:$binding'];
		if (point == null)
			throw 'the shader has no uniform block at group $group, binding $binding';
		return point;
	}

	/**
		The texture unit of the texture at `group` and `binding`. When the shader
		samples it with more than one sampler, `samplerGroup` and `samplerBinding`
		say which.
	**/
	public function textureUnit(group:Int, binding:Int, ?samplerGroup:Int, ?samplerBinding:Int):Int {
		if (samplerGroup != null)
			return found(unitKey(group, binding, {group: samplerGroup, binding: samplerBinding}), group, binding);
		var prefix = '$group:$binding/';
		var matches = [for (key => unit in units) if (StringTools.startsWith(key, prefix)) unit];
		if (matches.length > 1)
			throw 'the texture at group $group, binding $binding is sampled with ${matches.length} samplers: name one';
		if (matches.length == 0)
			throw 'the shader has no texture at group $group, binding $binding';
		return matches[0];
	}

	function found(key:String, group:Int, binding:Int):Int {
		var unit = units[key];
		if (unit == null)
			throw 'the shader has no texture at group $group, binding $binding with that sampler';
		return unit;
	}

	static function unitKey(group:Int, binding:Int, sampler:Null<{group:Int, binding:Int}>):String {
		return '$group:$binding/' + (sampler == null ? "" : '${sampler.group}:${sampler.binding}');
	}
}
#end
