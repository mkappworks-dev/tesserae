// The chunk material: StandardMaterial lit by bevy_pbr, tinted by the Voxel's
// Material index and gated by the flood-fill's two light channels.
//
// A vertex stage is needed as well as a fragment stage only because bevy_pbr's
// VertexOutput is a fixed struct: there is no way to forward an extra per-vertex
// attribute through pbr's own vertex entry point.

import bevy_pbr::render::{
    forward_io::{FragmentOutput, VertexOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::{
        STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS,
        STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    },
};
import super::{
    leaf_hole::leaf_hole,
    chunk_vertex::{ChunkVertex, ChunkVertexOutput, transform_vertex},
    material_texture::{material_slot, has_texture, sample_texture},
};

// Bindings 0-99 are the base StandardMaterial's. CHUNK_PALETTE_LEN,
// CHUNK_PALETTE_SLOTS and CHUNK_FLAME are injected by
// ChunkMaterialExtension::specialize. The slot after the Materials' is the
// flames'.
@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> palette: array<vec4<f32>, constants::CHUNK_PALETTE_SLOTS>;

// Mirrors LodParams in chunk_material.rs.
struct LodParams {
    tint: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var<uniform> lod: LodParams;

// Mirrors client::drawn_mask, which documents the layout.
@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var<storage, read> drawn: array<u32>;

// How far behind a face its own Voxel is sampled, as a fraction of the finer
// Cell: a thirty-second of one of its Voxels, far enough to clear the float
// error in a world-space coordinate and far short of leaving the Voxel.
const BEHIND_A_FACE: f32 = 1.0 / 1024.0;

// Whether `level` draws the Chunk holding the face at `point` facing `normal`.
// The Chunk asked about is the one behind the face, because a face lies on the
// boundary of the Voxel it is the surface of: water levelled onto a Chunk
// boundary by `level_the_water` would otherwise ask the Chunk above it, which
// holds no water at all.
fn drawn_at(level: u32, point: vec3<f32>, normal: vec3<f32>) -> bool {
    let slot = 1u + 8u * level;
    let size = vec3<u32>(drawn[slot + 4u], drawn[slot + 5u], drawn[slot + 6u]);
    if min(size.x, min(size.y, size.z)) == 0u {
        return false;
    }
    let origin = vec3<i32>(
        bitcast<i32>(drawn[slot + 1u]),
        bitcast<i32>(drawn[slot + 2u]),
        bitcast<i32>(drawn[slot + 3u]),
    );
    let cell_m = bitcast<f32>(drawn[slot + 7u]);
    let behind = point - normalize(normal) * cell_m * BEHIND_A_FACE;
    let local = vec3<i32>(floor(behind / cell_m)) - origin;
    if any(local < vec3<i32>(0)) || any(vec3<u32>(local) >= size) {
        return false;
    }
    let l = vec3<u32>(local);
    let i = (l.z * size.y + l.y) * size.x + l.x;
    return (drawn[drawn[slot] + i / 32u] & (1u << (i % 32u))) != 0u;
}

// Whether a level finer than `level` draws the Chunk at `point`, which then
// draws it instead.
fn finer_drawn(level: u32, point: vec3<f32>, normal: vec3<f32>) -> bool {
    for (var finer = 0u; finer < min(level, drawn[0]); finer++) {
        if drawn_at(finer, point, normal) {
            return true;
        }
    }
    return false;
}

const TINT_STRENGTH: f32 = 0.55;

const TINT_LEVELS: u32 = 6u;

// How a 0-15 light level becomes radiance. Minecraft's own curve, close enough:
// each level down is a fixed fraction of the one above, which keeps the dim end
// of the range usable instead of crushing it the way a linear ramp does.
const LIGHT_FALLOFF: f32 = 0.8;

// A lantern's colour. Warm, because every light a player places is a flame
// until a Mod says otherwise. A WESL const and not a uniform: step 9's
// look-pack API is where a pack gets to choose this.
const BLOCK_LIGHT_TINT: vec3<f32> = vec3<f32>(1.0, 0.72, 0.42);

// How bright a full-strength block light is, in the same units bevy_pbr's
// exposure works in.
const BLOCK_LIGHT_STRENGTH: f32 = 4.0;

// Flat, like the Material index: every vertex of a greedy quad carries the same
// level because light is part of the merge key, so interpolating would only
// blur the seam between two quads that deliberately did not merge.
fn light_level(packed: u32, shift: u32) -> f32 {
    let level = (packed >> shift) & constants::CHUNK_MAX_LIGHT;
    // Level 0 is not 0.8^15, it is nothing. Without this, a sealed cave would
    // sit at four percent of full brightness and never reach black.
    if level == 0u {
        return 0.0;
    }
    return pow(LIGHT_FALLOFF, f32(constants::CHUNK_MAX_LIGHT - level));
}

fn level_tint(level: u32) -> vec3<f32> {
    var tints = array<vec3<f32>, TINT_LEVELS>(
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(1.0, 0.2, 0.2),
        vec3<f32>(1.0, 0.85, 0.1),
        vec3<f32>(0.2, 1.0, 0.2),
        vec3<f32>(0.1, 0.9, 1.0),
        vec3<f32>(0.6, 0.3, 1.0),
    );
    return tints[min(level, TINT_LEVELS - 1u)];
}

@vertex
fn vertex(in: ChunkVertex) -> ChunkVertexOutput {
    return transform_vertex(in);
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

    // Coarse geometry up to a Voxel above the real surface would stand over the
    // finer silhouette, and coarse water under finer water would be drawn twice.
    if finer_drawn(in.level, in.world_position.xyz, in.world_normal) {
        discard;
    }

    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);

    // Multiplied rather than assigned so the base material's base_color still
    // works as a whole-chunk tint.
    let slot = material_slot(in.material);
    if has_texture(slot) {
        pbr_input.material.base_color *= sample_texture(slot, in.uv, in.world_normal);
    } else {
        pbr_input.material.base_color *= palette[slot];
    }

    if lod.tint != 0u && in.level != 0u {
        pbr_input.material.base_color = vec4<f32>(
            mix(pbr_input.material.base_color.rgb, level_tint(in.level), TINT_STRENGTH),
            pbr_input.material.base_color.a,
        );
    }

    // Sky light is how much of the sky dome and the sun this face can see, so it
    // drives bevy_pbr's own two occlusion inputs rather than being multiplied
    // into base_color the way Minecraft does it. That keeps albedo as albedo and
    // leaves the BRDF working.
    //
    // Both of them, not one. diffuse_occlusion alone would leave the sun
    // leaking into a roofed space, because the shadow map has no idea the roof
    // is there once the fragment is past the last cascade;
    // directional_shadow_factor alone would leave the ambient dome lighting a
    // cave. pbr_types.wesl's own doc on that field says "use this for custom
    // shadow maps", which is exactly what a sky-light flood-fill is.
    let sky = light_level(in.light, 4u);
    pbr_input.diffuse_occlusion *= vec3<f32>(sky);
    pbr_input.directional_shadow_factor *= sky;

    // Block light is a local source, so it is emissive: tinted by the Voxel's
    // own palette colour, so a face a lantern reaches still shows its Material
    // instead of turning into a flat glow. Alpha 1.0 puts it through exposure
    // with the rest of the lighting.
    let block = light_level(in.light, 0u);
    // rgb only: emissive.a selects whether exposure applies, and adding to it
    // would push it past 1.0, where bevy_pbr's mix() extrapolates.
    pbr_input.material.emissive = vec4<f32>(
        pbr_input.material.emissive.rgb
            + pbr_input.material.base_color.rgb * BLOCK_LIGHT_TINT * block * BLOCK_LIGHT_STRENGTH,
        pbr_input.material.emissive.a,
    );

    let alpha_mode = pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK && !has_texture(slot) && leaf_hole(in.uv) {
        pbr_input.material.base_color.a = 0.0;
    }

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
