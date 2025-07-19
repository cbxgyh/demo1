// 梯度减法着色器（速度场修正）
@group(0) @binding(0) var pressure: texture_2d<f32>;
@group(0) @binding(1) var velocity: texture_2d<f32>;
@group(0) @binding(2) var wind: texture_2d<f32>;
@group(0) @binding(3) var cells: texture_2d<f32>;
@group(0) @binding(4) var output: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(5) var sampler_pressure: sampler;
@group(0) @binding(6) var sampler_velocity: sampler;
@group(0) @binding(7) var sampler_wind: sampler;
@group(0) @binding(8) var sampler_cells: sampler;

struct GradientSubtractUniforms {
    texel_size: vec2<f32>,
    wind_strength: f32,  // 风力系数
    damping: f32,        // 阻尼系数
};

@group(0) @binding(9) var<uniform> gradient_subtract_uniforms: GradientSubtractUniforms;

// 边界处理函数
fn boundary(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv,vec2<f32>(0.0),vec2<f32>( 1.0));
}

// 计算相邻像素坐标
fn get_neighbor_uv(uv: vec2<f32>, direction: vec2<i32>) -> vec2<f32> {
    return boundary(uv + vec2<f32>(direction) * gradient_subtract_uniforms.texel_size);
}
//  gradient_subtract
@compute @workgroup_size(8, 8)
fn gradient_subtract_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(velocity);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }

    let uv = vec2<f32>(global_id.xy) / vec2<f32>(size);

    // 计算压力梯度（相邻压力差）
    let L = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(-1, 0)),0.).x;
    let R = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(1, 0)),0.).x;
    let T = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(0, 1)),0.).x;
    let B = textureSampleLevel(pressure, sampler_pressure, get_neighbor_uv(uv, vec2(0, -1)),0.).x;

    // 采样当前速度、风力和单元格类型
    let vel = textureSampleLevel(velocity, sampler_velocity, uv,0.).xy;
    let wind = textureSampleLevel(wind, sampler_wind, uv,0.).xy;
    let cell = textureSampleLevel(cells, sampler_cells, uv,0.).xy;

    // 1. 压力梯度减法（使流体不可压缩）
    var new_vel = vel - vec2(R - L, T - B);

    // 2. 应用风力（可能需要调整方向）
    new_vel += wind * gradient_subtract_uniforms.wind_strength;

    // 3. 根据单元格类型修改速度
    let cell_type = i32(cell.r * 255.0 + 0.5);

    // 固体边界（速度设为0）
    if (cell_type == 1 || cell_type == 5) {
        new_vel = vec2(0.0);
    }
    // 特殊处理类型0、4、6（保持当前速度）
    else if (cell_type != 0 && cell_type != 4 && cell_type != 6) {
        // 其他类型应用阻尼
        new_vel *= gradient_subtract_uniforms.damping;
    }
//    result=vec4<f32>(1.0,1.1,1.0,1);
    textureStore(output, vec2<i32>(global_id.xy), vec4<f32>(new_vel, 0.0, 1.0));
//    textureStore(output, vec2<i32>(global_id.xy), result);
}