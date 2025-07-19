// 涡度应用着色器（添加涡度约束）
@group(0) @binding(0) var velocity: texture_2d<f32>;
@group(0) @binding(1) var curl: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var sampler_velocity: sampler;
@group(0) @binding(3) var sampler_curl: sampler;

struct VorticityUniforms {
    texel_size: vec2<f32>,
    curl_strength: f32,  // 涡度强度系数
    dt: f32,             // 时间步长
};

@group(0) @binding(4) var<uniform> vorticity_uniforms: VorticityUniforms;

// 计算相邻像素坐标
fn get_neighbor_uv(uv: vec2<f32>, direction: vec2<i32>) -> vec2<f32> {
    return clamp(
            uv + vec2<f32>(direction) * vorticity_uniforms.texel_size,
            vec2<f32>(0.0),
            vec2<f32>(1.0)
        );
}
//  vorticity
@compute @workgroup_size(8, 8)
fn vorticity_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(velocity);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }

    let uv = vec2<f32>(global_id.xy) / vec2<f32>(size);

    // 采样当前位置和相邻位置的涡度
    let T = textureSampleLevel(curl, sampler_curl, get_neighbor_uv(uv, vec2(0, 1)),0.).x;
    let B = textureSampleLevel(curl, sampler_curl, get_neighbor_uv(uv, vec2(0, -1)),0.).x;
    let C = textureSampleLevel(curl, sampler_curl, uv,0.).x;

    // 计算涡度力（涡度平流）
    // 力的方向垂直于涡度梯度方向
    var force = vec2(abs(T) - abs(B), 0.0);

    // 归一化并应用涡度强度
    let length_force = length(force) + 0.00001;  // 避免除以零
    force *= (vorticity_uniforms.curl_strength * C) / length_force;

    // 采样当前速度并添加涡度力
    let vel = textureSampleLevel(velocity, sampler_velocity, uv,0.).xy;
    let new_vel = vel + force * vorticity_uniforms.dt;

    textureStore(output, vec2<i32>(global_id.xy), vec4<f32>(new_vel, 0.0, 1.0));
}