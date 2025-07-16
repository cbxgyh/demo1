// 速度场修正着色器（范围限制与归一化）
@group(0) @binding(0) var velocity: texture_2d<f32>;
@group(0) @binding(1) var pressure: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var sampler_linear: sampler;

struct VelocityOutUniforms {
    min_value: f32,      // 最小值限制
    max_value: f32,      // 最大值限制
    scale_factor: f32,   // 缩放因子
    offset: vec2<f32>,   // 偏移量
};

@group(0) @binding(4) var<uniform> velocity_out_uniforms: VelocityOutUniforms;
//  velocity_out
@compute @workgroup_size(8, 8)
fn velocity_out_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(velocity);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }

    let uv = vec2<f32>(global_id.xy) / vec2<f32>(size);

    // 采样速度和压力
    let v = textureSampleLevel(velocity, sampler_linear, uv,0.).rg;
    let p = textureSampleLevel(pressure, sampler_linear, uv,0.).r;

    // 合并为vec3 (vx, vy, pressure)
    var vp = vec3<f32>(v, p);

    // 1. 范围限制（防止数值爆炸）
    vp = clamp(vp, vec3<f32>(velocity_out_uniforms.min_value), vec3<f32>(velocity_out_uniforms.max_value));

    // 2. 归一化到[-1, 1]范围
    vp /= velocity_out_uniforms.scale_factor;

    // 3. 偏移到[0, 1]范围（便于渲染）
    vp += vec3<f32>(velocity_out_uniforms.offset, 0.0);

    // 输出修正后的速度场
    textureStore(output, vec2<i32>(global_id.xy), vec4<f32>(vp, 0.0));
}
