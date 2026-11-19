// Forward, depth and shadow passes use the same Material id and face mapping.
import bevy_pbr::render::{
    mesh_bindings::mesh,
    mesh_functions,
    view_transformations::position_world_to_clip,
};

// Every Chunk layer's words, as client::chunk_mesh documents them: a header,
// a table of (Material, light) words, then a record per quad, laid out as
// tesserae_core::mesh::Quad documents. Positions are in sixteenths of a Voxel
// and the entity's transform scales Voxels to metres.
@group(constants::MATERIAL_BIND_GROUP) @binding(106)
var<storage, read> quads: array<u32>;

// The mesh is a placeholder with four vertices a quad and nothing in them,
// drawn through its indices, two triangles a quad.
struct ChunkVertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
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

const WIDE: u32 = 3u;

// One vertex in the layer's own space. chunk_mesh::expand is this, step for
// step, and a test holds it equal to the mesher's corners.
struct Expanded {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    material: u32,
    light: u32,
    level: u32,
}

fn expand(at: u32, vertex: u32) -> Expanded {
    let table = at + constants::CHUNK_HEADER_WORDS;
    let stride = quads[at + 2u];
    let corner = vertex % 4u;
    let record = table + quads[at + 1u] + (vertex / 4u) * stride;
    let lo = quads[record];
    let hi = quads[record + 1u];
    var paint: u32;
    if stride == WIDE {
        paint = quads[record + 2u];
    } else {
        paint = quads[table + (hi >> constants::CHUNK_PAINT_SHIFT)];
    }

    var out: Expanded;
    out.position = vec3<f32>(0.0);
    out.normal = vec3<f32>(0.0);
    let kind = lo & 7u;
    if kind < constants::CHUNK_KIND_CROSS {
        let positive = (kind & 1u) == 1u;
        let d = kind / 2u;
        let u = (d + 1u) % 3u;
        let v = (d + 2u) % 3u;
        let plane = i32((lo >> 3u) & 1023u) - constants::CHUNK_PLANE_BIAS;
        let u0 = (lo >> 13u) & 511u;
        let v0 = (lo >> 22u) & 511u;
        let u1 = u0 + (hi & 511u) + 1u;
        let v1 = v0 + ((hi >> 9u) & 511u) + 1u;
        // Wound counter-clockwise seen from the face's side.
        let middle = corner == 1u || corner == 2u;
        let high_u = select(corner >= 2u, middle, positive);
        let high_v = select(middle, corner >= 2u, positive);
        let cu = select(u0, u1, high_u);
        let cv = select(v0, v1, high_v);
        out.position[d] = f32(plane) / 16.0;
        out.position[u] = f32(cu) / 16.0;
        out.position[v] = f32(cv) / 16.0;
        out.normal[d] = select(-1.0, 1.0, positive);
        // From the Voxel the quad starts in, so a tiling sampler repeats per Voxel.
        out.uv = vec2<f32>(f32(cu - u0 / 16u * 16u), f32(cv - v0 / 16u * 16u)) / 16.0;
    } else {
        let c = lo >> 3u;
        let cell = vec3<u32>(c & 31u, (c >> 5u) & 31u, (c >> 10u) & 31u);
        let reversed = ((lo >> 20u) & 1u) == 1u;
        let i = select(corner, 3u - corner, reversed);
        let high_u = i == 1u || i == 2u;
        let high_v = i >= 2u;
        out.uv = vec2<f32>(f32(high_u), f32(high_v));
        let side = select(1.0, -1.0, reversed);
        if kind == constants::CHUNK_KIND_CROSS {
            let rising = ((lo >> 18u) & 1u) == 0u;
            let x = select(!high_u, high_u, rising);
            out.position = vec3<f32>(cell + vec3<u32>(u32(x), u32(high_v), u32(high_u)));
            out.normal.z = side;
        } else {
            let d = (lo >> 18u) & 3u;
            let u = (d + 1u) % 3u;
            let v = (d + 2u) % 3u;
            let offset = (lo >> 21u) & 31u;
            let cu = select(hi & 15u, ((hi >> 4u) & 15u) + 1u, high_u);
            let cv = select((hi >> 8u) & 15u, ((hi >> 12u) & 15u) + 1u, high_v);
            out.position[d] = f32(cell[d] * 16u + offset) / 16.0;
            out.position[u] = f32(cell[u] * 16u + cu) / 16.0;
            out.position[v] = f32(cell[v] * 16u + cv) / 16.0;
            out.normal[d] = side;
        }
    }
    out.material = paint & ((1u << constants::CHUNK_LIGHT_SHIFT) - 1u);
    out.light = paint >> constants::CHUNK_LIGHT_SHIFT;
    out.level = quads[at];
    return out;
}

fn transform_vertex(in: ChunkVertex) -> ChunkVertexOutput {
    // The tag is where the layer's words start; the vertex, less the mesh's
    // own first vertex, is the index the placeholder drew it by.
    let at = mesh_functions::get_tag(in.instance_index);
    let vertex = in.vertex_index - mesh[in.instance_index].first_vertex_index;
    let quad = expand(at, vertex);

    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);

    var out: ChunkVertexOutput;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(quad.position, 1.0),
    );
    out.clip_position = position_world_to_clip(out.world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        quad.normal,
        in.instance_index,
    );
    out.uv = quad.uv;
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.clip_position.z;
    out.clip_position.z = min(out.clip_position.z, 1.0);
}
    out.instance_index = in.instance_index;
    out.material = quad.material;
    out.light = quad.light;
    out.level = quad.level;
    return out;
}
