
@group(0) @binding(0) var uTexture: texture_2d<f32>;
@group(0) @binding(1) var uWind: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var sampler_uTexture: sampler;
@group(0) @binding(4) var sampler_uWind: sampler;


struct ClearUniforms {
    value: f32,
};
@group(0) @binding(5) var<uniform> clear_uniforms: ClearUniforms;

@compute @workgroup_size(8, 8)
fn clear_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= u32(textureDimensions(output).x) ||
        global_id.y >= u32(textureDimensions(output).y)) {
        return;
    }

    let vUv = vec2<f32>(
        f32(global_id.x) / f32(textureDimensions(output).x),
        f32(global_id.y) / f32(textureDimensions(output).y)
    );

    var pressure = textureSampleLevel(uWind, sampler_uWind, vUv,0.0).z;
    pressure*=512.;
    pressure*=pressure;
    var result = clear_uniforms.value * (textureSampleLevel(uTexture, sampler_uTexture, vUv,0.0) + vec4<f32>(pressure, 0.0, 0.0, 0.0));
    textureStore(output, vec2<i32>(global_id.xy), result);
}