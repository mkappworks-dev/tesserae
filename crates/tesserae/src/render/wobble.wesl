// The view under water, after the scene is drawn and before tonemapping: each pixel is read from
// a point a little off it that drifts in time, so the whole view shimmers, and the water's colour
// is laid lightly over it.

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

struct Wobble {
    time_s: f32,
    // How far a pixel is read off, as a share of the view's height.
    strength: f32,
    // How much of the water's colour is laid over the view.
    tint: f32,
    hue: vec3<f32>,
}

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> wobble: Wobble;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(screen));
    let aspect = size.x / size.y;
    let p = in.uv * vec2<f32>(aspect, 1.0);
    let t = wobble.time_s;
    // Two slow waves across each other, so the shimmer has no single direction.
    let off = vec2<f32>(
        sin(p.y * 17.0 + t * 1.9) + 0.5 * sin(p.x * 11.0 - t * 1.3),
        cos(p.x * 15.0 + t * 1.6) + 0.5 * cos(p.y * 13.0 + t * 1.1),
    ) * (wobble.strength / 1.5) * vec2<f32>(1.0 / aspect, 1.0);
    let uv = clamp(in.uv + off, vec2<f32>(0.0), vec2<f32>(1.0));
    let seen = textureSample(screen, screen_sampler, uv);
    return vec4<f32>(mix(seen.rgb, wobble.hue, wobble.tint), seen.a);
}
