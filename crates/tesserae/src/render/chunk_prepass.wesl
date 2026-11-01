// Texture alpha and coverage must cut the same holes in colour, depth and
// shadow passes.
import super::{
    chunk_vertex::{ChunkVertex, ChunkVertexOutput, transform_vertex},
    material_texture::{material_slot, has_texture, sample_texture},
    leaf_hole::leaf_hole,
};

@if(PREPASS_FRAGMENT)
import bevy_pbr::prepass::io::FragmentOutput;
@if(CHUNK_COVERAGE_CUT)
import super::coverage::finer_drawn;

@vertex
fn vertex(in: ChunkVertex) -> ChunkVertexOutput {
    return transform_vertex(in);
}

fn cut(in: ChunkVertexOutput) {
@if(CHUNK_COVERAGE_CUT) {
    if finer_drawn(in.level, in.world_position.xyz, in.world_normal) {
        discard;
    }
}
@if(MAY_DISCARD) {
    let slot = material_slot(in.material);
    if has_texture(slot) {
        if sample_texture(slot, in.uv, in.world_normal).a < 0.5 {
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
