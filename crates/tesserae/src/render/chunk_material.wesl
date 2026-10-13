// The chunk material: StandardMaterial lit by bevy_pbr, tinted by the Voxel's
// Material index and gated by the flood-fill's two light channels.
//
// A vertex stage is needed as well as a fragment stage only because bevy_pbr's
// VertexOutput is a fixed struct: there is no way to forward an extra per-vertex
// attribute through pbr's own vertex entry point.

import bevy_pbr::render::{
    forward_io::{FragmentOutput, VertexOutput},
    mesh_functions,
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    view_transformations::position_world_to_clip,
};

// Bindings 0-99 are the base StandardMaterial's. CHUNK_PALETTE_LEN is injected
// by ChunkMaterialExtension::specialize.
@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> palette: array<vec4<f32>, constants::CHUNK_PALETTE_LEN>;

// Mirrors LodParams in chunk_material.rs.
struct LodParams {
    voxel_size: f32,
    tint: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var<uniform> lod: LodParams;

// How far a coarse vertex is pushed away from the camera along its view ray, as
// a fraction of its level's Voxel, so that where level k+1 overlaps level k by a
// Cell the finer level's coplanar face wins the depth test. The push does not
// move the vertex on screen. Ticket 08 tunes it against the captures.
const BIAS_FRACTION: f32 = 0.2;

// How strongly the debug tint replaces a Material's colour.
const TINT_STRENGTH: f32 = 0.55;

// The debug tint of each level, 0 being untinted. Levels past the last reuse it.
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

// Locations 0-7 are bevy_pbr's; the greedy mesher's Material index takes 10 and
// its packed light 11. Attributes arrive uncompressed because
// chunk_mesh_to_mesh leaves Bevy's per-attribute vertex compression off.
// Turning it on needs decompress_vertex here.
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
}

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
    let world_from_local = mesh_functions::get_world_from_local(in.instance_index);

    let level = mesh_functions::get_tag(in.instance_index);

    var out: ChunkVertexOutput;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(in.position, 1.0),
    );
    // Only the clip position is pushed back: lighting and shadows keep the
    // vertex where it really is.
    let ray = normalize(out.world_position.xyz - view.world_position);
    let bias_m = BIAS_FRACTION * lod.voxel_size * exp2(f32(level));
    out.clip_position = position_world_to_clip(out.world_position.xyz + ray * bias_m);
    out.world_normal = mesh_functions::mesh_normal_local_to_world(in.normal, in.instance_index);
    out.uv = in.uv;
    out.instance_index = in.instance_index;
    out.material = in.material;
    out.light = in.light;
    out.level = level;
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
