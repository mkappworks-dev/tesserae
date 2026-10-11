// The chunk material: StandardMaterial lit by bevy_pbr, tinted by the Voxel's
// Material index.
//
// A vertex stage is needed as well as a fragment stage only because bevy_pbr's
// VertexOutput is a fixed struct: there is no way to forward an extra per-vertex
// attribute through pbr's own vertex entry point.

import bevy_pbr::render::{
    forward_io::{FragmentOutput, VertexOutput},
    mesh_functions,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    view_transformations::position_world_to_clip,
};

// Bindings 0-99 are the base StandardMaterial's. CHUNK_PALETTE_LEN is injected
// by ChunkMaterialExtension::specialize.
@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> palette: array<vec4<f32>, constants::CHUNK_PALETTE_LEN>;

// Locations 0-7 are bevy_pbr's; the greedy mesher's Material index takes 10.
// Attributes arrive uncompressed because chunk_mesh_to_mesh leaves Bevy's
// per-attribute vertex compression off. Turning it on needs decompress_vertex
// here.
struct ChunkVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(10) material: u32,
}

// Mirrors bevy_pbr's VertexOutput, which the fragment stage rebuilds from this.
struct ChunkVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(6) @interpolate(flat) instance_index: u32,
    @location(10) @interpolate(flat) material: u32,
}

@vertex
fn vertex(in: ChunkVertex) -> ChunkVertexOutput {
    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);

    var out: ChunkVertexOutput;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(in.position, 1.0),
    );
    out.clip_position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(in.normal, in.instance_index);
    out.uv = in.uv;
    out.instance_index = in.instance_index;
    out.material = in.material;
    return out;
}

@fragment
fn fragment(
    in: ChunkVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var vertex_output: VertexOutput;
    vertex_output.position = in.clip_position;
    vertex_output.world_position = in.world_position;
    vertex_output.world_normal = in.world_normal;
    vertex_output.uv = in.uv;
    vertex_output.instance_index = in.instance_index;

    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);

    // Multiplied rather than assigned so the base material's base_color still
    // works as a whole-chunk tint.
    pbr_input.material.base_color *= palette[min(in.material, constants::CHUNK_PALETTE_LEN - 1u)];
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
