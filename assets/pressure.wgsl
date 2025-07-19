// 压力求解着色器（Gauss-Seidel迭代法）
@group(0) @binding(0) var pressure: texture_2d<f32>;
@group(0) @binding(1) var divergence: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var sampler_pressure: sampler;
@group(0) @binding(4) var sampler_divergence: sampler;

struct PressureUniforms {
    texel_size: vec2<f32>,
    alpha: f32,       // 松弛因子（通常为1.0）
    reciprocal_beta: f32,  // 1.0 / (2.0 * (1.0 / h^2))
};

@group(0) @binding(5) var<uniform> pressure_uniforms: PressureUniforms;

// 边界处理函数
fn boundary(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv, vec2<f32>(0.0),vec2<f32>( 1.0));
}

// 计算相邻像素坐标
fn get_neighbor_uv(uv: vec2<f32>, direction: vec2<i32>) -> vec2<f32> {
    return boundary(uv + vec2<f32>(direction) * pressure_uniforms.texel_size);
}
//  pressure
@compute @workgroup_size(8, 8)
fn pressure_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(pressure);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }

    let uv = vec2<f32>(global_id.xy) / vec2<f32>(size);

    // 采样相邻压力值
    let L = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(-1, 0)),0.).x;
    let R = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(1, 0)),0.).x;
    let T = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(0, 1)),0.).x;
    let B = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(0, -1)),0.).x;

    // 当前压力值和散度
    let C = textureSampleLevel(pressure, sampler_pressure, uv,0.).x;
    let div = textureSampleLevel(divergence, sampler_divergence, uv,0.).x;

    // 压力方程求解（Gauss-Seidel迭代）
    // p = (neighbors_sum - divergence) * 0.25
    // 或更通用形式：p = (neighbors_sum + alpha * div) * reciprocal_beta
    let pressure = (L + R + B + T - div) * 0.25;

//    textureStore(output, vec2<i32>(global_id.xy), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    textureStore(output, vec2<i32>(global_id.xy), vec4<f32>(pressure, 0.0, 0.0, 1.0));
}