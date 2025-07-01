// 散度计算着色器
@group(0) @binding(0) var velocity: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var sampler_linear: sampler;

struct DivergenceUniforms {
    texel_size: vec2<f32>,
};

@group(0) @binding(3) var<uniform> divergence_uniforms: DivergenceUniforms;

// 计算相邻像素坐标
fn get_neighbor_uv(uv: vec2<f32>, direction: vec2<i32>) -> vec2<f32> {
    return uv + vec2<f32>(direction) * divergence_uniforms.texel_size;
}
//  divergence
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(velocity);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }

    let uv = vec2<f32>(global_id.xy) / vec2<f32>(size);

    // 采样相邻像素速度
    let vL = get_neighbor_uv(uv, vec2(-1, 0));  // 左
    let vR = get_neighbor_uv(uv, vec2(1, 0));   // 右
    let vT = get_neighbor_uv(uv, vec2(0, 1));   // 上
    let vB = get_neighbor_uv(uv, vec2(0, -1));  // 下

    let vel_L = textureSampleLevel(velocity, sampler_linear, vL,0.).x;
    let vel_R = textureSampleLevel(velocity, sampler_linear, vR,0.).x;
    let vel_T = textureSampleLevel(velocity, sampler_linear, vT,0.).y;
    let vel_B = textureSampleLevel(velocity, sampler_linear, vB,0.).y;

    // 计算散度
    let divergence = 0.5 * (vel_R - vel_L + vel_T - vel_B);

    // 输出结果（归一化到0-1范围）
    let color = vec4<f32>(divergence, 0.0, 0.0, 1.0);
    textureStore(output, vec2<i32>(global_id.xy), color);
}