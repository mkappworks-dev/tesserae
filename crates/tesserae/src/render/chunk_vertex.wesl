// Forward, depth and shadow passes use the same Material id and face mapping.
import bevy_pbr::render::{mesh_functions, view_transformations::position_world_to_clip};

// Mirrors client::chunk_mesh, which documents the packing. Positions and UVs
// are in steps of a Voxel and the entity's transform scales Voxels to metres.
struct ChunkVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position_face: vec4<i32>,
    @location(1) uv: vec2<u32>,
    @location(2) material_light: u32,
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

// A face in FACE_STEPS order: -x, +x, -y, +y, -z, +z.
fn face_normal(face: i32) -> vec3<f32> {
    var normal = vec3<f32>(0.0);
    normal[face / 2] = select(-1.0, 1.0, face % 2 == 1);
    return normal;
}

fn transform_vertex(in: ChunkVertex) -> ChunkVertexOutput {
    let steps = f32(constants::CHUNK_VERTEX_STEPS);
    let position = vec3<f32>(in.position_face.xyz) / steps;

    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);

    let level = mesh_functions::get_tag(in.instance_index);

    var out: ChunkVertexOutput;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(position, 1.0),
    );
    out.clip_position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        face_normal(in.position_face.w),
        in.instance_index,
    );
    out.uv = vec2<f32>(in.uv) / steps;
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.clip_position.z;
    out.clip_position.z = min(out.clip_position.z, 1.0);
}
    out.instance_index = in.instance_index;
    out.material = in.material_light & ((1u << constants::CHUNK_LIGHT_SHIFT) - 1u);
    out.light = in.material_light >> constants::CHUNK_LIGHT_SHIFT;
    out.level = level;
    return out;
}
