// 顶点着色器
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
// 片段着色器
@group(0) @binding(0) var output_tex: texture_2d<f32>;
@group(0) @binding(1) var output_sampler: sampler;

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {


        // 3. Calculate UV coordinates using floating-point division
        let uv =in.uv;

    // 或者更简洁地：
    // let uv = position.xy / vec2(tex_size.xy);

    return textureSample(output_tex, output_sampler, uv);
}