// 流体平流计算着色器
@group(0) @binding(0) var velocity: texture_2d<f32>;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var wind: texture_2d<f32>;
@group(0) @binding(3) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(4) var sampler_linear: sampler;

struct AdvectionUniforms {
    texel_size: vec2<f32>,
    dt: f32,
    dissipation: f32,
};

@group(0) @binding(5) var<uniform> advection_uniforms: AdvectionUniforms;
// advection
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x >= u32(textureDimensions(output).x) ||
        global_id.y >= u32(textureDimensions(output).y)) {
        return;
    }

    let vUv = vec2<f32>(
        f32(global_id.x) / f32(textureDimensions(output).x),
        f32(global_id.y) / f32(textureDimensions(output).y)
    );

    let coord = vUv - advection_uniforms.dt * textureSampleLevel(velocity, sampler_linear, vUv,0.0).xy * advection_uniforms.texel_size;
    let density = textureSampleLevel(wind, sampler_linear, vUv,0.0).w * 1.0;

    var result = advection_uniforms.dissipation * (textureSampleLevel(source, sampler_linear, coord,0.0) + vec4<f32>(density, 0.0, 0.0, 0.0));
    result.a = 1.0;

    textureStore(output, vec2<i32>(global_id.xy), result);
}  