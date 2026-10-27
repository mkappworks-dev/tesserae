import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct Icon {
    frame: vec4<f32>,
    opacity: f32,
}
@group(1) @binding(0)
var<uniform> icon: Icon;
@group(1) @binding(1)
var atlas: texture_2d<f32>;
@group(1) @binding(2)
var atlas_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = icon.frame.xy + in.uv * icon.frame.zw;
    return textureSample(atlas, atlas_sampler, uv) * icon.opacity;
}
