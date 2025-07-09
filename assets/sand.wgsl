
// 顶点着色器 vertex.wgsl
struct VertexInput {
    @location(0) position: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.uv = input.position;
    output.position = vec4f(input.position, 0.0, 1.0);
    return output;
}
// 片段着色器 fragment.wgsl
struct ShaderParams {
    time: f32,
    dpi: f32,
    resolution: vec2f,
    is_snapshot: u32,
};

@group(2) @binding(0) var<uniform> params: ShaderParams;
//@group(2) @binding(1) var data_tex: texture_storage_2d<rgba8unorm, read_write>;
@group(2) @binding(1) var data_tex: texture_2d<f32>;
@group(2) @binding(2) var tex_sampler: sampler;

const PI: f32 = 3.141592653589793;

fn hsv2rgb(hsv: vec3<f32>) -> vec3<f32> {
    // 实现 hsv 到 rgb 的转换
    // 示例实现，可能不准确
    let c = hsv.z * hsv.y;
    let x = c * (1.0 - abs(mod1(hsv.x * 6.0, 2.0) - 1.0));
    let m = hsv.z - c;
    if (hsv.x < 1.0 / 6.0) {
        return vec3<f32>(c + m, x + m, m);
    } else if (hsv.x < 2.0 / 6.0) {
        return vec3<f32>(x + m, c + m, m);
    } else if (hsv.x < 3.0 / 6.0) {
        return vec3<f32>(m, c + m, x + m);
    } else if (hsv.x < 4.0 / 6.0) {
        return vec3<f32>(m, x + m, c + m);
    } else if (hsv.x < 5.0 / 6.0) {
        return vec3<f32>(x + m, m, c + m);
    } else {
        return vec3<f32>(c + m, m, x + m);
    }
}
fn mod1(a: f32, b: f32) -> f32 {
    return a - b * floor(a / b);
}

fn random(pos: vec2f) -> f32 {
    return fract(sin(dot(pos, vec2f(12.9898, 78.233))) * 43758.5453);
}

fn snoise2(pos: vec2f) -> f32 {
    let i = floor(pos);
    let f = fract(pos);

    // 简化版的噪声实现
    let a = random(i);
    let b = random(i + vec2f(1.0, 0.0));
    let c = random(i + vec2f(0.0, 1.0));
    let d = random(i + vec2f(1.0, 1.0));

    let u = f * f * (3.0 - 2.0 * f);
    return mix(a, b, u.x) + (c - a)* u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

fn snoise3(pos: vec3f) -> f32 {
    return snoise2(pos.xy) * 0.8 + snoise2(pos.yz) * 0.2;
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4f {// 修改text_coord计算（原代码存在坐标翻转问题）


   let text_coord = vec2f(
      (uv.x + 1.0) * 0.5,
              (uv.y + 1.0) * 0.5 // Y轴翻转
   );
    // 精确采样
    let tex_size = textureDimensions(data_tex);
    let coord = vec2<i32>(text_coord * vec2<f32>(tex_size));
    let data = textureLoad(data_tex, coord, 0);

    // 类型转换
    let type_id = i32(round(data.r * 255.0));

    var hue: f32 = 0.0;
    var saturation: f32 = 0.6;
    var lightness: f32 = 0.3 + data.g * 0.5;
    var a: f32 = 0.6;

    // 计算公共噪声值
    let pixel_pos = floor(uv * params.resolution / params.dpi);
    let noise3 = snoise3(vec3f(pixel_pos, params.time * 0.05));
    let noise2 = snoise2(pixel_pos);

    switch(type_id) {
        case 0: { // Empty
            hue = 0.0;
            saturation = 0.1;
            lightness = 0.1;
            a = 0.1;
            if (params.is_snapshot != 0u) {
                saturation = 0.05;
                lightness = 1.01;
                a = 1.0;
            }
        }
        case 1: { // Wall
            hue = 0.1;
            saturation = 0.1;
            lightness = 0.4;
        }
        case 2: { // Sand
            hue = 0.1;
            saturation = 0.5;
            lightness += 0.3;
        }
        case 3: { // Water
            hue = 0.6;
            lightness = 0.7 + data.g * 0.25 + noise3 * 0.1;
            let polarity = i32(data.g * 255.0) % 2;
            if (polarity == 0) {
                lightness += 0.01;
            }
        }
        case 4: { // Gas
            hue = 0.0;
            lightness += 0.4;
            saturation = 0.2 + data.b * 1.5;
        }
        case 5: { // Cloner
            hue = 0.9;
            saturation = 0.3;
        }
        case 6: { // Fire
            hue =  0.0;
            saturation = 0.9;
            lightness = 0.7 + data.g * 0.3;
            if (params.is_snapshot != 0u) {
                lightness -= 0.2;
            }
            return vec4f(hsv2rgb(vec3f(hue, saturation, lightness)), 1.0);
        }
        case 7: { // Wood
            hue = data.g * 0.1;
            saturation = 0.3;
            lightness = 0.3 + data.g * 0.3;
        }
        case 8: { // Lava
            hue = data.g * 0.1;
            lightness = 0.7 + data.g * 0.25 + noise3 * 0.1;
        }
        case 9: { // Ice
            hue = 0.6;
            saturation = 0.4;
            lightness = 0.7 + data.g * 0.5;
        }
        case 10: { // Sink
            hue = 0.9;
            saturation = 0.4;
            lightness = 1.0;
        }
        case 11: { // Plant
            hue = 0.4;
            saturation = 0.4;
        }
        case 12: { // Acid
            hue = 0.18;
            saturation = 0.9;
            lightness = 0.8 + data.g * 0.2 + noise3 * 0.05;
        }
        case 13: { // Stone
            hue = -0.4 + data.g * 0.5;
            saturation = 0.1;
        }
        case 14: { // Dust
            hue = data.g * 2.0 + params.time * 0.0008;
            saturation = 0.4;
            lightness = 0.8;
        }
        case 15: { // Mite
            hue = 0.8;
            saturation = 0.9;
            lightness = 0.8;
        }
        case 16: { // Oil
            hue = data.g * 5.0 + params.time * 0.008;
            saturation = 0.2;
            lightness = 0.3;
        }
        case 17: { // Rocket
            hue = 0.0;
            saturation = 0.4 + data.b;
            lightness = 0.9;
        }
        case 18: { // Fungus
            hue = data.g * 0.15 - 0.1;
            saturation = data.g * 0.8 - 0.05;
            lightness = 1.5 - data.g * 0.2;
        }
        case 19: { // Seed
            hue = fract(fract(data.b * 2.0) * 0.5) - 0.3;
            saturation = 0.7 * (data.g + 0.4) + data.b * 0.2;
            lightness = 0.9 * (data.g + 0.9);
        }
        default: {
            // 处理未知类型
            hue = 0.0;
            saturation = 1.0;
            lightness = 1.0;
            a = 1.0;
        }
    }

    // 动态噪声效果
    if (params.is_snapshot == 0u) {
        lightness *= 0.975 + noise2 * 0.025;
    }
    a = 0.7;
    let hsv = vec3f(hue, saturation, lightness);
    return vec4f(hsv2rgb(hsv), a);
}