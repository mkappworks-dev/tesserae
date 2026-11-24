// Texture alpha must cut the same holes in colour, depth and shadow passes.
import super::{
    chunk_vertex::{ChunkVertex, ChunkVertexOutput, NO_AREA, transform_vertex, vertex_level},
    material_texture::{material_slot, has_texture, sample_texture, uv_gradient},
    leaf_hole::leaf_hole,
};

@if(PREPASS_FRAGMENT)
import bevy_pbr::prepass::io::FragmentOutput;

@vertex
fn vertex(in: ChunkVertex) -> ChunkVertexOutput {
    // Only the camera's prepass writes normals, and only screen-space AO reads them, which cannot
    // reach past level 0. Level 0 is the finest, so no coverage cut is needed either; the main
    // pass draws the coarse levels over this depth, and a shadow pass still draws them all.
@if(NORMAL_PREPASS) {
    if vertex_level(in) > 0u {
        var out: ChunkVertexOutput;
        out.clip_position = NO_AREA;
        return out;
    }
}
    return transform_vertex(in);
}

fn cut(in: ChunkVertexOutput) {
@if(MAY_DISCARD) {
    let uv_grad = uv_gradient(in.uv);
    let slot = material_slot(in.material);
    if has_texture(slot) {
        if sample_texture(slot, in.uv, uv_grad, in.world_normal).a < 0.5 {
            discard;
        }
    } else if leaf_hole(in.uv) {
        discard;
    }
}
}

@if(PREPASS_FRAGMENT)
@fragment
fn fragment(in: ChunkVertexOutput) -> FragmentOutput {
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
fn fragment(in: ChunkVertexOutput) {
    cut(in);
}
