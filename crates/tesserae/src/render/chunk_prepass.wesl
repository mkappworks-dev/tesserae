// The chunk material's depth and shadow passes. Only the cutout material may
// discard there, and where it does, its holes are cut, so a canopy's shadow
// has them too.

import bevy_pbr::prepass::io::VertexOutput;
import super::leaf_hole::leaf_hole;

// Declared only where the pass writes more than depth.
@if(PREPASS_FRAGMENT)
import bevy_pbr::prepass::io::FragmentOutput;

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
