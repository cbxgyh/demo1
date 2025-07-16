//
//@group(0) @binding(0) var uTexture: texture_2d<f32>;
//@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;
//@group(0) @binding(2) var sampler_linear: sampler;
//
//
//@compute @workgroup_size(8, 8)
//fn display_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
//    if (global_id.x >= u32(textureDimensions(output).x) ||
//        global_id.y >= u32(textureDimensions(output).y)) {
//        return;
//    }
//
//    let vUv = vec2<f32>(
//        f32(global_id.x) / f32(textureDimensions(output).x),
//        f32(global_id.y) / f32(textureDimensions(output).y)
//    );
////// 翻转Y并交换XY
//    var color = textureSampleLevel(uTexture, sampler_linear, vec2(1.0 - vUv.y,vUv.x),0.0).rgb * 0.1;
////    // 颜色处理流程
//    color *= 0.5;
//     // 钳制分量式
////      color = vec4<f32>(min(color.rgb, 0.9),1.0);
//    color =  min(color.rgb, vec3(0.9));
//     // 颜色反相
//    color = vec3(1.0) - color;
//     // 暖色调滤镜
//    color *= vec3(0.95, 0.9, 0.9);
//    textureStore(output, vec2<i32>(global_id.xy), vec4<f32>(color,1.0));
//}
//

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
@group(0) @binding(0) var uTexture: texture_2d<f32>;
@group(0) @binding(1) var sampler_linear: sampler;
@fragment
fn fragment(input: FullscreenVertexOutput) ->@location(0) vec4<f32>  {
    // 应用90度旋转（交换并反转坐标）
    let uv = vec2<f32>(1.0 - input.uv.y, input.uv.x);

    // 纹理采样
    var color = textureSample(uTexture, sampler_linear, uv).rgb * 0.1;

    // 颜色处理
    color *= 0.5;
    color = min(color.rgb, vec3(0.9));
    color = vec3<f32>(1.0) - color;
    color *= vec3<f32>(0.95, 0.9, 0.9);

    // 输出颜色
    return vec4<f32>(color,0.0);
}