// 流体旋度(curl)计算着色器
@group(0) @binding(0) var velocity: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var sampler_linear: sampler;

struct CurlUniforms {
    texel_size: vec2<f32>,
};

@group(0) @binding(3) var<uniform> curl_uniforms: CurlUniforms;
// curl
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

    // 计算相邻像素坐标
    let vL = vec2<f32>(vUv.x - curl_uniforms.texel_size.x, vUv.y);
    let vR = vec2<f32>(vUv.x + curl_uniforms.texel_size.x, vUv.y);
    let vT = vec2<f32>(vUv.x, vUv.y - curl_uniforms.texel_size.y);
    let vB = vec2<f32>(vUv.x, vUv.y + curl_uniforms.texel_size.y);

    // 采样速度场
    let L = textureSampleLevel(velocity, sampler_linear, vL,0.).y;
    let R = textureSampleLevel(velocity, sampler_linear, vR,0.).y;
    let T = textureSampleLevel(velocity, sampler_linear, vT,0.).x;
    let B = textureSampleLevel(velocity, sampler_linear, vB,0.).x;

    // 计算旋度
    let vorticity = R - L - T + B;

    // 输出结果
    textureStore(output, vec2<i32>(global_id.xy), vec4<f32>(vorticity, 0.0, 0.0, 1.0));
}