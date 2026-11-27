// The chunk material: StandardMaterial lit by bevy_pbr, tinted by the Voxel's
// Material index and gated by the flood-fill's two light channels.
//
// The vertex stage is its own: it expands each corner from the quad buffer, and
// bevy_pbr's VertexOutput is a fixed struct with no room for the Material and
// light it forwards.

import bevy_pbr::render::{
    forward_io::{FragmentOutput, VertexOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::{
        STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED,
        STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS, STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT,
        STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    },
    mesh_view_bindings as view_bindings,
};
@if(DISTANCE_FOG)
import bevy_pbr::render::fog::{atmospheric_fog, exponential_squared_fog};
@if(DISTANCE_FOG)
import bevy_pbr::render::mesh_view_types::FOG_MODE_ATMOSPHERIC;
@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
import bevy_pbr::render::mesh_view_bindings::{screen_space_ambient_occlusion_texture, view};
@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
import bevy_pbr::ssao::utils::ssao_multibounce;
import super::{
    leaf_hole::leaf_hole,
    chunk_vertex::{ChunkVertex, ChunkVertexOutput, transform_vertex},
    material_texture::{material_slot, has_texture, patch_colour, patched, sample_texture, uv_gradient},
    water::{
        WATER_REFLECTANCE, ambient_specular, column_depth_m, column_point, flow_cycle, fresnel, is_water,
        sky_reflection, water_body, water_depth_m, water_flow, water_foam_share, water_gradient,
        water_surface,
    },
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

// `drawn_mask::SOME` and `drawn_mask::ALL`, which a test holds equal.
const SOME: u32 = 1u;
const ALL: u32 = 2u;

// The cover bits of the level-`level` Chunk holding the face at `point` facing
// `normal`: `SOME`, its own level or a finer one draws part of it; `ALL`, all
// of it.
// The Chunk asked about is the one behind the face, because a face lies on the
// boundary of the Voxel it is the surface of: water levelled onto a Chunk
// boundary by `level_the_water` would otherwise ask the Chunk above it, which
// holds no water at all.
fn cover_at(level: u32, point: vec3<f32>, normal: vec3<f32>) -> u32 {
    let slot = 1u + 8u * level;
    let size = vec3<u32>(drawn[slot + 4u], drawn[slot + 5u], drawn[slot + 6u]);
    if min(size.x, min(size.y, size.z)) == 0u {
        return 0u;
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
        return 0u;
    }
    let l = vec3<u32>(local);
    let i = (l.z * size.y + l.y) * size.x + l.x;
    return (drawn[drawn[slot] + i / 16u] >> ((i % 16u) * 2u)) & (SOME | ALL);
}

// Whether a level finer than `level` draws the Chunk at `point`, which then
// draws it instead. Usually one lookup: only a partly covered Cell sends the
// walk a level further down.
fn finer_drawn(level: u32, point: vec3<f32>, normal: vec3<f32>) -> bool {
    var finer = min(level, drawn[0]);
    while finer > 0u {
        finer -= 1u;
        let cover = cover_at(finer, point, normal);
        if cover == 0u {
            return false;
        }
        if (cover & ALL) != 0u {
            return true;
        }
    }
    return false;
}

// Ripples calm toward the shore over this depth.
const CALM_DEPTH_M: f32 = 1.0;

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
// exposure works in: 4.0 on screen at Bevy's default Exposure (EV100 9.7).
const BLOCK_LIGHT_STRENGTH: f32 = 3992.0;

// Guards the division below: a black channel of the ambient shows no ambient at all, floor or not.
const MIN_AMBIENT: f32 = 1e-3;

// The share of the hour's ambient that is the noon floor a face no sky light reaches keeps
// (`tesserae_core::sky::CAVE_FLOOR`), so a sealed cave looks the same at every hour: bevy_pbr
// multiplies the occlusion by the hour's ambient, and this divides it back out. Per channel, so a
// blue night does not tint the cave either.
fn cave_floor() -> vec3<f32> {
    let noon = bitcast<f32>(constants::CHUNK_CAVE_FLOOR) * bitcast<f32>(constants::CHUNK_NOON_AMBIENT);
    return noon / max(view_bindings::lights.ambient_color.rgb, vec3<f32>(MIN_AMBIENT));
}

// The share of screen-space occlusion the sun's light takes as well as the sky's.
const SUN_OCCLUSION: f32 = 0.5;

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

// `render::fog::Haze::opacity_at`: aerial perspective in metres toward the
// fog colour, before Bevy's reach-sized fade, which then fades it in turn.
fn apply_haze(color: vec4<f32>, world_position: vec3<f32>) -> vec4<f32> {
    let distance = length(world_position - view_bindings::view.world_position.xyz);
    let rate = vec3<f32>(
        bitcast<f32>(constants::CHUNK_HAZE_RATE_R),
        bitcast<f32>(constants::CHUNK_HAZE_RATE_G),
        bitcast<f32>(constants::CHUNK_HAZE_RATE_B),
    );
    let fog = view_bindings::fog.base_color;
    let haze = bitcast<f32>(constants::CHUNK_HAZE_MOST) * (1.0 - exp(-distance * rate)) * fog.a;
    return vec4<f32>(mix(color.rgb, fog.rgb, haze), color.a);
}

// bevy_pbr's own fog samples the shadow map again for this glow, a second full shadow lookup per
// pixel for about 0.3 ms at 1080p; unshadowed, as the sky's and the clouds' glow is, it shows no
// difference: the glow is strongest far out, past the last cascade, where Bevy's lookup is 1 anyway.
fn fog_glow(direction: vec3<f32>) -> vec3<f32> {
    let fog = view_bindings::fog;
    if fog.directional_light_color.a <= 0.0 {
        return vec3<f32>(0.0);
    }
    var glow = vec3<f32>(0.0);
    for (var i: u32 = 0u; i < view_bindings::lights.n_directional_lights; i = i + 1u) {
        let light = view_bindings::lights.directional_lights[i];
        let toward = max(dot(direction, light.direction_to_light), 0.0);
        glow += pow(toward, fog.directional_light_exponent) * light.color.rgb;
    }
    return glow * view_bindings::view.exposure;
}

// fog.rs always sets ExponentialSquared in air; under water, `client::underwater` sets
// Atmospheric, which fades each channel at its own rate and takes no haze.
fn apply_fog(color: vec4<f32>, world_position: vec3<f32>) -> vec4<f32> {
    let toward = world_position - view_bindings::view.world_position.xyz;
    let distance = length(toward);
    if view_bindings::fog.mode == FOG_MODE_ATMOSPHERIC {
        return atmospheric_fog(view_bindings::fog, color, distance, vec3<f32>(0.0));
    }
    let hazed = apply_haze(color, world_position);
    return exponential_squared_fog(view_bindings::fog, hazed, distance, fog_glow(toward / distance));
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
    vertex_output.instance_index = in.instance_index;
    let uv_grad = uv_gradient(in.uv);
    let water_grad = water_gradient(in.world_position.xyz);

    // Coarse geometry up to a Voxel above the real surface would stand over the
    // finer silhouette, and coarse water under finer water would be drawn twice.
    @if(!CHUNK_NO_COVERAGE_DISCARD)
    if finer_drawn(in.level, in.world_position.xyz, in.world_normal) {
        discard;
    }

    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);

    // Multiplied rather than assigned so the base material's base_color still
    // works as a whole-chunk tint.
    let slot = material_slot(in.material);
    let water = is_water(slot);
    // Water's tile stays for Items, the inventory and the coarse colour; its surface is its own.
    if has_texture(slot) && !water {
        pbr_input.material.base_color *= sample_texture(slot, in.uv, uv_grad, in.world_normal, in.voxel);
        if patched(slot, in.world_normal) {
            pbr_input.material.base_color = vec4<f32>(
                patch_colour(pbr_input.material.base_color.rgb, in.world_position.xyz),
                pbr_input.material.base_color.a,
            );
        }
    } else {
        pbr_input.material.base_color *= palette[slot];
    }

    if lod.tint != 0u && in.level != 0u {
        pbr_input.material.base_color = vec4<f32>(
            mix(pbr_input.material.base_color.rgb, level_tint(in.level), TINT_STRENGTH),
            pbr_input.material.base_color.a,
        );
    }

    let sky = light_level(in.light, 4u);

    // bevy_pbr bounces the occlusion off StandardMaterial's white base_color, before the Material's
    // colour is multiplied in, which lifts it to almost nothing; bounce it off the real albedo.
    // Water is not in the prepass AO reads, so it would take the floor's occlusion.
    @if(SCREEN_SPACE_AMBIENT_OCCLUSION) {
    if in.level == 0u && !water {
        let ssao_at = vec2<i32>(in.clip_position.xy - view.viewport.xy);
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, ssao_at, 0i).r;
        // Faded by sky light, which already darkens what cannot see the sky: a shaded floor is
        // not occluded twice.
        let bounced = ssao_multibounce(ssao, pbr_input.material.base_color.rgb);
        pbr_input.diffuse_occlusion = mix(vec3<f32>(1.0), bounced, sky);
        // A look choice, not physics: sunlit faces are lit almost wholly by the sun, so occlusion
        // on the ambient term alone would leave a canopy's creases as flat as before.
        pbr_input.directional_shadow_factor *= mix(1.0, ssao, SUN_OCCLUSION);
    } else {
        // Only level 0 is in the prepass AO reads: bevy_pbr read an empty texel for this face.
        pbr_input.diffuse_occlusion = vec3<f32>(1.0);
        pbr_input.specular_occlusion = 1.0;
    }
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
    // cave past its faint floor. pbr_types.wesl's own doc on that field says "use this for custom
    // shadow maps", which is exactly what a sky-light flood-fill is.
    // The dome share scales the sky only, so a cave's faint floor stays as it was.
    pbr_input.diffuse_occlusion *= max(vec3<f32>(sky * in.sky_view), cave_floor());
    pbr_input.directional_shadow_factor *= sky;

    // Premultiplied: what the water sends toward the eye, as `alpha` of the colour below is lost.
    // The body's share is scaled into its albedo, so bevy_pbr's lighting leaves the sun's glint
    // and the sky's reflection unscaled by the body's alpha.
    var reflection = vec3<f32>(0.0);
    var water_alpha = 1.0;
    if water {
        // From under the surface there is no sky to reflect, and the depth is not what the view
        // crosses; the underwater fog does the rest. Coarse levels carry no columns.
        let from_below = pbr_input.world_normal.y < -0.5;
        let columns = in.water != 0u && !from_below;
        var depth_m = 0.0;
        var flow = vec2<f32>(0.0);
        var foam = 0.0;
        let cycle = flow_cycle(in.world_position.xz);
        if columns {
            let point = column_point(in.voxel, in.uv);
            // Depth, Flow and foam are the top's: a wall of water stands at its column's edge,
            // where the filtered depth runs out as at a shore, so it takes its own column's.
            if abs(pbr_input.world_normal.y) > 0.5 {
                depth_m = water_depth_m(in.water, point);
                flow = water_flow(in.water, point, in.world_position.xyz);
                foam = water_foam_share(in.world_position.xz, water_grad, depth_m, flow, cycle);
            } else {
                depth_m = column_depth_m(in.water, vec2<i32>(floor(point)));
            }
        }
        let calm = select(1.0, smoothstep(0.0, CALM_DEPTH_M, depth_m), columns);
        let surface = water_surface(
            slot,
            in.world_position.xyz,
            water_grad,
            pbr_input.N,
            calm,
            flow,
            cycle,
        );
        pbr_input.N = surface.normal;
        pbr_input.material.perceptual_roughness = mix(surface.perceptual_roughness, 1.0, foam);
        pbr_input.material.reflectance = vec3<f32>(WATER_REFLECTANCE);
        let facing = dot(surface.normal, pbr_input.V);
        let mirrored = select(fresnel(facing), 0.0, from_below) * (1.0 - foam);
        var albedo = pbr_input.material.base_color.rgb * pbr_input.material.base_color.a;
        var hides = pbr_input.material.base_color.a;
        if columns {
            let cos_view = abs(dot(pbr_input.world_normal, pbr_input.V));
            let body = water_body(slot, depth_m, cos_view, foam);
            albedo = body.albedo;
            hides = body.alpha;
        }
        pbr_input.material.base_color = vec4<f32>(albedo * (1.0 - mirrored), 1.0);
        reflection = mirrored * sky * sky_reflection(pbr_input.V, surface.normal);
        water_alpha = hides * (1.0 - mirrored) + mirrored;
    }

    // Block light is a local source, so it is emissive: tinted by the Voxel's
    // own palette colour, so a face a lantern reaches still shows its Material
    // instead of turning into a flat glow. Its exposure weight is 1, so it goes
    // through the camera's Exposure as the sun and the sky do, and a change of
    // Exposure keeps lanterns and daylight in step.
    let block = light_level(in.light, 0u);
    // emissive.a is that weight; past 1.0 bevy_pbr's mix() would extrapolate.
    pbr_input.material.emissive = vec4<f32>(
        pbr_input.material.emissive.rgb
            + pbr_input.material.base_color.rgb * BLOCK_LIGHT_TINT * block * BLOCK_LIGHT_STRENGTH,
        1.0,
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
    if water {
        // The sky's reflection replaces the ambient light's specular, which at a low angle is
        // nearly all of the ambient and turned the far water grey.
        let sent = out.color.rgb - ambient_specular(pbr_input) + reflection;
        // Back out of premultiplied, so fog and the tonemapper see a colour, as for glass.
        out.color = vec4<f32>(sent / water_alpha, water_alpha);
    }
    @if(DISTANCE_FOG)
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0u {
        out.color = apply_fog(out.color, in.world_position.xyz);
    }
    // Fogged above; Bevy's own fog would sample the shadow map again.
    pbr_input.material.flags &= ~STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    // bevy_pbr leaves a premultiplied material's colour as the shader writes it.
    if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        out.color = vec4<f32>(out.color.rgb * out.color.a, out.color.a);
    }
    return out;
}
