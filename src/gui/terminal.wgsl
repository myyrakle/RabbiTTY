struct Uniforms {
    cell_size : vec2<f32>,
    viewport  : vec2<f32>,
    offset    : vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms : Uniforms;

struct VertexIn {
    @location(0) quad_pos : vec2<f32>,
    @location(1) cell_pos : vec2<u32>,
    @location(2) color    : vec4<f32>,
};

struct VertexOut {
    @builtin(position) position : vec4<f32>,
    @location(0) color : vec4<f32>,
};

@vertex
fn vs_main(input : VertexIn) -> VertexOut {
    let cell = vec2<f32>(input.cell_pos);
    let pixel = (cell + input.quad_pos) * uniforms.cell_size + uniforms.offset;

    // Convert to NDC (origin top-left)
    let ndc = vec2<f32>(
        (pixel.x / uniforms.viewport.x) * 2.0 - 1.0,
        1.0 - (pixel.y / uniforms.viewport.y) * 2.0
    );

    var out : VertexOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input : VertexOut) -> @location(0) vec4<f32> {
    return input.color;
}

struct TextUniforms {
    viewport : vec2<f32>,
    offset   : vec2<f32>,
};

@group(1) @binding(0)
var<uniform> text_uniforms : TextUniforms;
@group(1) @binding(1)
var text_sampler : sampler;
@group(1) @binding(2)
var text_atlas : texture_2d<f32>;

struct TextVertexIn {
    @location(0) quad_pos   : vec2<f32>,
    @location(1) glyph_pos  : vec2<f32>,
    @location(2) glyph_size : vec2<f32>,
    @location(3) uv_min     : vec2<f32>,
    @location(4) uv_max     : vec2<f32>,
    @location(5) color      : vec4<f32>,
    @location(6) bg_color   : vec4<f32>,
};

struct TextVertexOut {
    @builtin(position) position : vec4<f32>,
    @location(0) uv : vec2<f32>,
    @location(1) color : vec4<f32>,
    @location(2) bg_color : vec4<f32>,
};

@vertex
fn text_vs_main(input : TextVertexIn) -> TextVertexOut {
    let pixel = input.glyph_pos + input.quad_pos * input.glyph_size + text_uniforms.offset;
    let ndc = vec2<f32>(
        (pixel.x / text_uniforms.viewport.x) * 2.0 - 1.0,
        1.0 - (pixel.y / text_uniforms.viewport.y) * 2.0
    );
    let uv = input.uv_min + (input.uv_max - input.uv_min) * input.quad_pos;

    var out : TextVertexOut;
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv;
    out.color = input.color;
    out.bg_color = input.bg_color;
    return out;
}

// Convert linear RGB to approximate sRGB (sqrt approximation of gamma 2.2)
fn linear_to_srgb(c : vec3<f32>) -> vec3<f32> {
    return sqrt(max(c, vec3<f32>(0.0)));
}

// Convert approximate sRGB back to linear
fn srgb_to_linear(c : vec3<f32>) -> vec3<f32> {
    return c * c;
}

// Single-pass subpixel: blend fg/bg in sRGB space for perceptually correct text
@fragment
fn text_fs_subpixel(input : TextVertexOut) -> @location(0) vec4<f32> {
    let cov = textureSample(text_atlas, text_sampler, input.uv);
    // Blend in sRGB (gamma) space so text edges look bold and vivid
    let bg_srgb = linear_to_srgb(input.bg_color.rgb);
    let fg_srgb = linear_to_srgb(input.color.rgb);
    let blended_srgb = bg_srgb * (1.0 - cov.rgb) + fg_srgb * cov.rgb;
    let blended = srgb_to_linear(blended_srgb);
    let alpha = max(cov.r, max(cov.g, cov.b));
    return vec4<f32>(blended, mix(input.bg_color.a, input.color.a, alpha));
}

struct CompositeVertexIn {
    @location(0) pos : vec2<f32>,
    @location(1) uv  : vec2<f32>,
};

struct CompositeVertexOut {
    @builtin(position) position : vec4<f32>,
    @location(0) uv : vec2<f32>,
};

@group(0) @binding(0)
var composite_sampler : sampler;
@group(0) @binding(1)
var composite_texture : texture_2d<f32>;

@vertex
fn composite_vs_main(input : CompositeVertexIn) -> CompositeVertexOut {
    var out : CompositeVertexOut;
    out.position = vec4<f32>(input.pos, 0.0, 1.0);
    out.uv = input.uv;
    return out;
}

@fragment
fn composite_fs_main(input : CompositeVertexOut) -> @location(0) vec4<f32> {
    return textureSample(composite_texture, composite_sampler, input.uv);
}
