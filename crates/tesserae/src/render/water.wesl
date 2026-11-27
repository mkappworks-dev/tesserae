// The water path of the chunk shader; see render/water.rs.

import bevy_pbr::render::{
    mesh_view_bindings::{globals, lights, view},
    pbr_lighting::{EnvBRDFApprox, F_AB},
    pbr_types::PbrInput,
};
import super::sky_colour::sky_before_tonemap;

// water::table's rows, three per palette slot.
@group(constants::MATERIAL_BIND_GROUP) @binding(107)
var<uniform> water_table: array<vec4<f32>, constants::CHUNK_WATER_ROWS>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108)
var water_normals: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(109)
var water_normals_sampler: sampler;

// Mirrors WaterSky in chunk_material.rs.
struct WaterSky {
    zenith: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(110)
var<uniform> water_sky: WaterSky;

// F0 0.02, water's reflectance at normal incidence from its index of refraction of 1.33, as
// bevy_pbr's `reflectance`: F0 = 0.16 reflectance².
const WATER_F0: f32 = 0.02;
const WATER_REFLECTANCE: f32 = 0.35355339;

// `water::LONGEST_CRESTS`: the most crests across one repeat of the normal map.
const MOST_CRESTS: f32 = 14.0;

// The roughness water rises to where its ripples are all smaller than a pixel.
const FAR_ROUGHNESS: f32 = 0.3;

// Seconds into the clock's period. A capture pair holds it, so the pair differs by the Eye's step
// alone.
fn clock() -> f32 {
    var now = globals.time;
    @if(CHUNK_WATER_CLOCK_HELD) {
        now = bitcast<f32>(constants::CHUNK_WATER_CLOCK_S);
    }
    return now;
}

fn is_water(slot: u32) -> bool {
    return water_table[slot * 3u].w > 0.5;
}

fn roughness_of(slot: u32) -> f32 {
    return water_table[slot * 3u + 1u].w;
}

// The world's x and z's screen-space derivatives. Taken at the top of the fragment stage, before
// any discard or per-slot branch: derivatives need the whole 2x2 quad.
struct WaterGradient {
    ddx: vec2<f32>,
    ddy: vec2<f32>,
}

fn water_gradient(world_position: vec3<f32>) -> WaterGradient {
    return WaterGradient(dpdx(world_position.xz), dpdy(world_position.xz));
}

// One layer's slope and the slope variance its mips averaged away, which becomes roughness.
struct Ripples {
    slope: vec2<f32>,
    lost: f32,
}

fn ripple_layer(
    xz: vec2<f32>,
    gradient: WaterGradient,
    tile_m: f32,
    tiles_per_period: vec2<f32>,
    strength: f32,
) -> Ripples {
    // A whole number of repeats per period, so this is continuous where `globals.time` wraps.
    let drift = clock() * (tiles_per_period / f32(constants::CHUNK_CLOCK_PERIOD_S));
    let uv = xz / tile_m + drift;
    let ddx = gradient.ddx / tile_m;
    let ddy = gradient.ddy / tile_m;
    // How many of the layer's shortest crests one pixel spans.
    let crests = max(length(ddx), length(ddy)) * MOST_CRESTS;
    let lost = smoothstep(0.25, 2.0, crests);
    let slope = textureSampleGrad(water_normals, water_normals_sampler, uv, ddx, ddy).rg * strength;
    // A sine's slope variance is half its peak squared.
    return Ripples(slope * (1.0 - lost), 0.5 * strength * strength * lost);
}

struct WaterSurface {
    normal: vec3<f32>,
    perceptual_roughness: f32,
}

// `normal` faces the viewer, as bevy_pbr's double-sided flip leaves it. Ripples move only the
// tops and undersides; a wall of water keeps its face's normal.
fn water_surface(
    slot: u32,
    world_position: vec3<f32>,
    gradient: WaterGradient,
    normal: vec3<f32>,
) -> WaterSurface {
    let base = roughness_of(slot);
    if abs(normal.y) < 0.5 {
        return WaterSurface(normal, base);
    }
    let a = ripple_layer(
        world_position.xz,
        gradient,
        bitcast<f32>(constants::CHUNK_RIPPLE0_TILE_M),
        vec2<f32>(f32(constants::CHUNK_RIPPLE0_X), f32(constants::CHUNK_RIPPLE0_Z)),
        bitcast<f32>(constants::CHUNK_RIPPLE0_STRENGTH),
    );
    let b = ripple_layer(
        // Turned against the first, so the two layers' texels never line up.
        vec2<f32>(world_position.z, -world_position.x),
        WaterGradient(vec2<f32>(gradient.ddx.y, -gradient.ddx.x), vec2<f32>(gradient.ddy.y, -gradient.ddy.x)),
        bitcast<f32>(constants::CHUNK_RIPPLE1_TILE_M),
        vec2<f32>(f32(constants::CHUNK_RIPPLE1_X), f32(constants::CHUNK_RIPPLE1_Z)),
        bitcast<f32>(constants::CHUNK_RIPPLE1_STRENGTH),
    );
    let turned_back = vec2<f32>(-b.slope.y, b.slope.x);
    let slope = a.slope + turned_back;
    let up = sign(normal.y);
    let rippled = normalize(vec3<f32>(-slope.x, 1.0, -slope.y) * vec3<f32>(1.0, up, 1.0));
    // Beckmann's alpha squared is twice the slope variance; bevy_pbr's alpha is roughness squared.
    let alpha = sqrt(base * base * base * base + 2.0 * (a.lost + b.lost));
    return WaterSurface(rippled, min(sqrt(alpha), max(base, FAR_ROUGHNESS)));
}

fn fresnel(cos_view: f32) -> f32 {
    return WATER_F0 + (1.0 - WATER_F0) * pow(1.0 - clamp(cos_view, 0.0, 1.0), 5.0);
}

// The sky along `normal`'s reflection of the view, before tonemapping. A ripple can tip the ray
// below the horizon; it is mirrored back up rather than reflecting the ground's colour, which
// this has no view of.
fn sky_reflection(view_toward: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let ray = reflect(-view_toward, normal);
    return sky_before_tonemap(normalize(vec3<f32>(ray.x, abs(ray.y), ray.z)), water_sky.zenith.rgb);
}

// What bevy_pbr's `ambient_light` adds for water's specular, worked out as it does and scaled by
// the exposure as `apply_pbr_lighting` scales it, so the water path can take it back out.
fn ambient_specular(in: PbrInput) -> vec3<f32> {
    let f0 = vec3<f32>(WATER_F0);
    let n_dot_v = max(dot(in.N, in.V), 0.0001);
    let specular = EnvBRDFApprox(f0, F_AB(in.material.perceptual_roughness, n_dot_v));
    let occlusion = saturate(dot(f0, vec3<f32>(50.0 * 0.33)));
    return specular * occlusion * lights.ambient_color.rgb * in.diffuse_occlusion * view.exposure;
}
