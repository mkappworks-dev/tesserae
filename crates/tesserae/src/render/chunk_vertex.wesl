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

// The mesh is its bucket's placeholder with four vertices a quad and nothing in
// them, drawn through its indices, two triangles a quad.
struct ChunkVertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
}

// Mirrors bevy_pbr's VertexOutput, which the fragment stage rebuilds from this.
// CHUNK_SHADOW, the sun's shadow pass, keeps only what its alpha test reads.
struct ChunkVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @if(!CHUNK_SHADOW)
    @location(0) world_position: vec4<f32>,
    @if(!CHUNK_SHADOW || MAY_DISCARD)
    @location(1) world_normal: vec3<f32>,
    @if(!CHUNK_SHADOW || MAY_DISCARD)
    @location(2) uv: vec2<f32>,
    @if(!CHUNK_SHADOW)
    @location(6) @interpolate(flat) instance_index: u32,
    @if(!CHUNK_SHADOW || MAY_DISCARD)
    @location(10) @interpolate(flat) material: u32,
    @if(!CHUNK_SHADOW)
    @location(11) @interpolate(flat) light: u32,
    @if(!CHUNK_SHADOW)
    @location(12) @interpolate(flat) level: u32,
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    @location(13) unclipped_depth: f32,
}

// The record's fields, as tesserae_core::mesh::layout lays them out.
const KIND_MASK: u32 = (1u << constants::CHUNK_KIND_BITS) - 1u;
const PLANE_MASK: u32 = (1u << constants::CHUNK_PLANE_BITS) - 1u;
const CORNER_MASK: u32 = (1u << constants::CHUNK_CORNER_BITS) - 1u;
const CELL_MASK: u32 = (1u << constants::CHUNK_CELL_BITS) - 1u;
const AXIS_MASK: u32 = (1u << constants::CHUNK_AXIS_BITS) - 1u;
const OFFSET_MASK: u32 = (1u << constants::CHUNK_OFFSET_BITS) - 1u;
const SPAN_MASK: u32 = (1u << constants::CHUNK_SPAN_BITS) - 1u;
const MATERIAL_MASK: u32 = (1u << constants::CHUNK_LIGHT_SHIFT) - 1u;
const SUB: u32 = constants::CHUNK_SUBDIV;
const UNIT: f32 = 1.0 / f32(constants::CHUNK_SUBDIV);

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

fn span(hi: u32, k: u32) -> u32 {
    return (hi >> (k * constants::CHUNK_SPAN_BITS)) & SPAN_MASK;
}

fn expand(at: u32, vertex: u32) -> Expanded {
    let table = at + constants::CHUNK_HEADER_WORDS;
    let stride = quads[at + constants::CHUNK_HEADER_STRIDE];
    let corner = vertex % constants::CHUNK_VERTICES_PER_QUAD;
    let record = table + quads[at + constants::CHUNK_HEADER_TABLE_LEN] + (vertex / constants::CHUNK_VERTICES_PER_QUAD) * stride;
    let lo = quads[record];
    let hi = quads[record + 1u];
    var paint: u32;
    if stride == constants::CHUNK_WIDE_STRIDE {
        paint = quads[record + 2u];
    } else {
        paint = quads[table + (hi >> constants::CHUNK_PAINT_SHIFT)];
    }

    var out: Expanded;
    out.position = vec3<f32>(0.0);
    out.normal = vec3<f32>(0.0);
    let kind = lo & KIND_MASK;
    if kind < constants::CHUNK_FACES {
        let positive = (kind & 1u) == 1u;
        let d = kind / 2u;
        let u = (d + 1u) % 3u;
        let v = (d + 2u) % 3u;
        let plane = i32((lo >> constants::CHUNK_PLANE_SHIFT) & PLANE_MASK) - constants::CHUNK_PLANE_BIAS;
        let u0 = (lo >> constants::CHUNK_U0_SHIFT) & CORNER_MASK;
        let v0 = (lo >> constants::CHUNK_V0_SHIFT) & CORNER_MASK;
        let u1 = u0 + (hi & CORNER_MASK) + 1u;
        let v1 = v0 + ((hi >> constants::CHUNK_DV_SHIFT) & CORNER_MASK) + 1u;
        // Wound counter-clockwise seen from the face's side.
        let middle = corner == 1u || corner == 2u;
        let high_u = select(corner >= 2u, middle, positive);
        let high_v = select(middle, corner >= 2u, positive);
        let cu = select(u0, u1, high_u);
        let cv = select(v0, v1, high_v);
        out.position[d] = f32(plane) * UNIT;
        out.position[u] = f32(cu) * UNIT;
        out.position[v] = f32(cv) * UNIT;
        out.normal[d] = select(-1.0, 1.0, positive);
        // From the Voxel the quad starts in, so a tiling sampler repeats per Voxel.
        out.uv = vec2<f32>(f32(cu - u0 / SUB * SUB), f32(cv - v0 / SUB * SUB)) * UNIT;
    } else {
        let c = lo >> constants::CHUNK_CELL_SHIFT;
        let bits: u32 = constants::CHUNK_CELL_BITS;
        let cell = vec3<u32>(c & CELL_MASK, (c >> bits) & CELL_MASK, (c >> (2u * bits)) & CELL_MASK);
        let reversed = ((lo >> constants::CHUNK_REVERSED_SHIFT) & 1u) == 1u;
        let i = select(corner, 3u - corner, reversed);
        let high_u = i == 1u || i == 2u;
        let high_v = i >= 2u;
        out.uv = vec2<f32>(f32(high_u), f32(high_v));
        let side = select(1.0, -1.0, reversed);
        if kind == constants::CHUNK_KIND_CROSS {
            let rising = ((lo >> constants::CHUNK_DIAGONAL_SHIFT) & 1u) == 0u;
            let x = select(!high_u, high_u, rising);
            out.position = vec3<f32>(cell + vec3<u32>(u32(x), u32(high_v), u32(high_u)));
            out.normal.z = side;
        } else {
            let d = (lo >> constants::CHUNK_AXIS_SHIFT) & AXIS_MASK;
            let u = (d + 1u) % 3u;
            let v = (d + 2u) % 3u;
            let offset = (lo >> constants::CHUNK_OFFSET_SHIFT) & OFFSET_MASK;
            let cu = select(span(hi, 0u), span(hi, 1u) + 1u, high_u);
            let cv = select(span(hi, 2u), span(hi, 3u) + 1u, high_v);
            out.position[d] = f32(cell[d] * SUB + offset) * UNIT;
            out.position[u] = f32(cell[u] * SUB + cu) * UNIT;
            out.position[v] = f32(cell[v] * SUB + cv) * UNIT;
            out.normal[d] = side;
        }
    }
    out.material = paint & MATERIAL_MASK;
    out.light = paint >> constants::CHUNK_LIGHT_SHIFT;
    out.level = quads[at + constants::CHUNK_HEADER_LEVEL];
    return out;
}

// The shadow pass's whole decode. Through `expand` rather than a decode of its
// own: Metal compiles with fast math, so any other shape of the same arithmetic
// may round the shadow map differently. The compiler drops the unused reads.
fn expand_position(at: u32, vertex: u32) -> vec3<f32> {
    return expand(at, vertex).position;
}

fn transform_vertex(in: ChunkVertex) -> ChunkVertexOutput {
    // The tag is where the layer's words start; the vertex, less the mesh's
    // own first vertex, is the index the placeholder drew it by.
    let at = mesh_functions::get_tag(in.instance_index);
    let vertex = in.vertex_index - mesh[in.instance_index].first_vertex_index;
    var out: ChunkVertexOutput;
    if vertex / constants::CHUNK_VERTICES_PER_QUAD >= quads[at + constants::CHUNK_HEADER_QUADS] {
        // The bucket's padding: every corner at one point outside the clip
        // volume, so its triangles have no area and nothing rasterizes.
        out.clip_position = vec4<f32>(2.0, 2.0, 0.5, 1.0);
        return out;
    }
@if(CHUNK_SHADOW && !MAY_DISCARD) {
    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);
    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(expand_position(at, vertex), 1.0),
    );
    out.clip_position = position_world_to_clip(world_position.xyz);
}
@else {
    let quad = expand(at, vertex);

    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);

    let world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(quad.position, 1.0),
    );
    out.clip_position = position_world_to_clip(world_position.xyz);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        quad.normal,
        in.instance_index,
    );
    out.uv = quad.uv;
    out.material = quad.material;
@if(!CHUNK_SHADOW) {
    out.world_position = world_position;
    out.instance_index = in.instance_index;
    out.light = quad.light;
    out.level = quad.level;
}
}
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.clip_position.z;
    out.clip_position.z = min(out.clip_position.z, 1.0);
}
    return out;
}
