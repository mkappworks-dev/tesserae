// The chunk material's depth and shadow passes. Only the cutout material may
// discard there, and where it does, its holes are cut, so a canopy's shadow
// has them too.

import bevy_pbr::prepass::io::VertexOutput;

// Declared only where the pass writes more than depth.
@if(PREPASS_FRAGMENT)
import bevy_pbr::prepass::io::FragmentOutput;

// The same as chunk_material.wesl's; the two must stay in step.
fn leaf_hole(uv: vec2<f32>) -> bool {
    let cell = vec2<u32>(floor(fract(uv) * 4.0));
    let voxel = vec2<u32>(vec2<i32>(floor(uv)) + vec2<i32>(4096));
    var h = (cell.x * 73856093u) ^ (cell.y * 19349663u) ^ (voxel.x * 83492791u) ^ (voxel.y * 2654435761u);
    h = (h ^ (h >> 13u)) * 1274126177u;
    return ((h >> 8u) & 1023u) < 360u;
}

fn cut(in: VertexOutput) {
@if(MAY_DISCARD) {
    if leaf_hole(in.uv) {
        discard;
    }
}
}

@if(PREPASS_FRAGMENT)
@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    cut(in);
    var out: FragmentOutput;

@if(NORMAL_PREPASS)
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);

@if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    out.frag_depth = in.unclipped_depth;

    return out;
}
@else
@fragment
fn fragment(in: VertexOutput) {
    cut(in);
}
