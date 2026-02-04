// Terminal shader for rendering quads with texture-based glyph rendering

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = in.color;
    return out;
}

@group(0) @binding(0)
var t_glyph: texture_2d<f32>;
@group(0) @binding(1)
var s_glyph: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Check if this is a textured glyph or solid color background
    // Background quads use tex_coord (0,0) to (1,1) but we set them to (0,0) for all corners
    // Glyph quads use actual UV coordinates from the atlas
    
    // If all tex coords are 0, this is a solid color (background/cursor)
    if (in.tex_coord.x == 0.0 && in.tex_coord.y == 0.0) {
        return in.color;
    }
    
    // Sample the glyph texture (R8 format - alpha/coverage in red channel)
    let alpha = textureSample(t_glyph, s_glyph, in.tex_coord).r;
    
    // Return the glyph color with sampled alpha
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
