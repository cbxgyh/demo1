@group(0) @binding(0) var uTexture: texture_2d<f32>;
@group(0) @binding(1) var uSampler: sampler;
@group(0) @binding(2) var<uniform> uDisplayMode: u32;

// 顶点着色器输出，片段着色器输入
struct FragmentInput {
    @location(0) vUv: vec2<f32>,
};


@fragment
fn fragment(input: FragmentInput) ->@location(0) vec4<f32> {
    // 应用90度旋转（交换并反转坐标）
    let uv = vec2<f32>(1.0 - input.vUv.y, input.vUv.x);

    // 纹理采样
    var color = textureSample(uTexture, uSampler, uv);

    // 根据显示模式处理颜色
    if (uDisplayMode == 0) {
        // 显示密度场（使用热图）
        color = vec4<f32>(color.r, color.r * 0.5, 0.0, 1.0);
    } else if (uDisplayMode == 1) {
        // 显示速度场（使用方向编码）
        let vel = color.xy;
        let speed = length(vel);
        let dir = vel / max(speed, 0.001);
        color = vec4<f32>((dir + 1.0) * 0.5, speed * 0.5, 1.0);
    } else if (uDisplayMode == 2) {
        // 显示旋度场（蓝色表示正旋度，红色表示负旋度）
        let curl = color.r;
        color = vec4<f32>(
            clamp(-curl, 0.0, 1.0),
            0.0,
            clamp(curl, 0.0, 1.0),
            1.0
        );
    } else if (uDisplayMode == 3) {
        // 显示散度场（绿色表示正散度，紫色表示负散度）
        let div = color.r;
        color = vec4<f32>(
            0.5 - div * 0.5,
            0.5,
            0.5 + div * 0.5,
            1.0
        );
    } else if (uDisplayMode == 4) {
        // 显示压力场（使用灰度）
        color = vec4<f32>(color.r, color.r, color.r, 1.0);
    } else {
        // 默认显示密度场
        color = vec4<f32>(color.r, color.r * 0.5, 0.0, 1.0);
    }

    // 颜色增强处理
    color =vec4<f32>(color.rgb * 0.1,1.0);
    color =vec4<f32>(color.rgb * 0.5,1.0);
//    color = vec4<f32>(min(color.rgb, 0.9),1.0);
//    color = vec4<f32>(vec3<f32>(1.0) - color.rgb, 0.9,1.0);

    // 输出颜色
    return color;

}