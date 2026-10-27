// Forward, depth and shadow passes use the same Material id and face mapping.
import bevy_pbr::render::{mesh_functions, view_transformations::position_world_to_clip};

struct ChunkVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(10) material: u32,
    @location(11) light: u32,
}

// Mirrors bevy_pbr's VertexOutput, which the fragment stage rebuilds from this.
struct ChunkVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(6) @interpolate(flat) instance_index: u32,
    @location(10) @interpolate(flat) material: u32,
    @location(11) @interpolate(flat) light: u32,
    @location(12) @interpolate(flat) level: u32,
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    @location(13) unclipped_depth: f32,
}

fn transform_vertex(in: ChunkVertex) -> ChunkVertexOutput {
    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);

    let level = mesh_functions::get_tag(in.instance_index);

    var out: ChunkVertexOutput;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(in.position, 1.0),
    );
    out.clip_position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(in.normal, in.instance_index);
    out.uv = in.uv;
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.clip_position.z;
    out.clip_position.z = min(out.clip_position.z, 1.0);
}
    out.instance_index = in.instance_index;
    out.material = in.material;
    out.light = in.light;
    out.level = level;
    return out;
}
