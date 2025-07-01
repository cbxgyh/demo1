// 输入纹理
@group(2) @binding(0) var uTexture: texture_2d<f32>;
@group(2) @binding(1) var uSampler: sampler;

// 顶点着色器输出，片段着色器输入
struct FragmentInput {
    @location(0) vUv: vec2<f32>,
};

//// 片段着色器输出
//struct FragmentOutput {
//    @location(0) color: vec4<f32>,
//};

@fragment
fn fragment(input: FragmentInput) -> vec4<f32>  {
    // 应用90度旋转（交换并反转坐标）
    let uv = vec2<f32>(1.0 - input.vUv.y, input.vUv.x);

    // 纹理采样
    var color = textureSample(uTexture, uSampler, uv).rgb * 0.1;

    // 颜色处理
    color *= 0.5;
    color = min(color, 0.9);
    color = vec3<f32>(1.0) - color;
    color *= vec3<f32>(0.95, 0.9, 0.9);

    // 输出颜色
    return color;
}