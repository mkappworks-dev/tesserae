// The sky gradient, the sun, the moon and the stars, behind everything; see render/sky.rs.

import bevy_pbr::render::mesh_view_bindings::view;
@if(TONEMAP_IN_SHADER) import bevy_core_pipeline::tonemapping::tone_mapping;
import super::sky_colour::{horizon, zenith_share};

// Mirrors SkyMaterial in sky.rs.
struct Sky {
    zenith: vec4<f32>,
    toward_sun: vec4<f32>,
    toward_moon: vec4<f32>,
    moon_light: vec4<f32>,
    stars_from_world: mat4x4<f32>,
    stars: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> sky: Sky;

// Larger than the real sun's quarter degree, which is under two pixels at the
// default field of view.
const SUN_RADIUS_RAD: f32 = 0.012;
const SUN_EDGE_RAD: f32 = 0.003;

// Linear and above 1, so the tonemapper rolls it off to a warm white.
const SUN_COLOUR: vec3<f32> = vec3<f32>(5.0, 4.6, 3.9);

// A wide faint glow and a tight bright one round the disc.
const GLOW_COLOUR: vec3<f32> = vec3<f32>(1.0, 0.9, 0.7);
const GLOW_WIDE: f32 = 0.22;
const GLOW_WIDE_POWER: f32 = 12.0;
const GLOW_TIGHT: f32 = 0.5;
const GLOW_TIGHT_POWER: f32 = 600.0;
// The glow is gone once the sun is this far below the horizon, as the sine of its depth: about 6°.
const GLOW_SET: f32 = 0.1;

// About three times the real moon's, a little more than the sun disc, so its phase reads at a
// glance.
const MOON_RADIUS_RAD: f32 = 0.014;
// Linear and above 1, a cool white once tonemapped, as the sun's is a warm one.
const MOON_COLOUR: vec3<f32> = vec3<f32>(1.5, 1.55, 1.7);
// The share of the lit face kept at the limb.
const MOON_LIMB: f32 = 0.7;
// How wide the terminator is, as the cosine of the light's angle to the surface.
const MOON_TERMINATOR: f32 = 0.08;
const MOON_GLOW: f32 = 0.06;
const MOON_GLOW_POWER: f32 = 2000.0;
// Where the glow has faded to nothing, as the cosine of the angle from the moon: about 8°.
const MOON_GLOW_FROM: f32 = 0.99;
// The disc fades into the horizon's haze, as the terrain at the reach fades into the fog: half
// gone 1° up, so the reach's edge never shows against it. The stars fade with the horizon band.
const MOON_HAZE: f32 = 40.0;

// Cells per cube face edge, each of about 1° and holding at most one star, a few thousand in all.
const STAR_CELLS: f32 = 90.0;
// The share of cells with a star.
const STAR_CHANCE: f32 = 0.16;
// A star stays this share of its cell away from the cell's edges, so a pixel only ever needs its
// own cell's star: Bevy's 45° view 1080 pixels high gives a star 0.08° of reach, under 0.15°.
const STAR_MARGIN: f32 = 0.15;
// How far a star's light reaches, in pixels: always a few pixels whatever the resolution, so a
// star turning across the pixel grid keeps its brightness and never shimmers.
const STAR_REACH_PX: f32 = 2.0;
const STAR_BRIGHTNESS: f32 = 1.4;
// A full moon high up takes this much off every star, so the faint two thirds vanish and the bright
// ones dim; a new moon or a set one takes nothing.
const MOON_WASH: f32 = 0.15;

const PI: f32 = 3.14159265358979;

struct Vertex {
    @location(0) position: vec3<f32>,
}

struct SkyOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

// The covering triangle's corners are already clip coordinates; z 0 is the far
// plane of Bevy's reverse-Z projection.
@vertex
fn vertex(in: Vertex) -> SkyOutput {
    var out: SkyOutput;
    out.clip = vec4<f32>(in.position.xy, 0.0, 1.0);
    out.ndc = in.position.xy;
    return out;
}

// What the tonemapper shows `colour` as, so the sky can be mixed in screen
// colour: the horizon as the fogged terrain shows it, the zenith as authored.
fn seen(colour: vec3<f32>) -> vec3<f32> {
    var out = vec4<f32>(colour, 1.0);
    @if(TONEMAP_IN_SHADER) {
        out = tone_mapping(out, view.color_grading);
    }
    return out.rgb;
}

fn hash(x: u32) -> u32 {
    // PCG, after Jarzynski and Olano's "Hash Functions for GPU Rendering".
    let state = x * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn unit(bits: u32) -> f32 {
    return f32(bits & 0xffffu) / 65535.0;
}

// The light of the one star in the cube-map cell `toward` falls in, if it holds one. The face is
// warped equi-angular, so cells are near enough the same size that no face's corners crowd.
fn star_light(toward: vec3<f32>, pixel_rad: f32, washed: f32) -> vec3<f32> {
    let a = abs(toward);
    var face: u32;
    var across: vec2<f32>;
    if a.x >= a.y && a.x >= a.z {
        face = select(1u, 0u, toward.x > 0.0);
        across = toward.yz / a.x;
    } else if a.y >= a.z {
        face = select(3u, 2u, toward.y > 0.0);
        across = toward.xz / a.y;
    } else {
        face = select(5u, 4u, toward.z > 0.0);
        across = toward.xy / a.z;
    }
    let warped = (atan(across) * (4.0 / PI) + 1.0) * (0.5 * STAR_CELLS);
    let cell = clamp(floor(warped), vec2<f32>(0.0), vec2<f32>(STAR_CELLS - 1.0));
    let id = (face * u32(STAR_CELLS) + u32(cell.y)) * u32(STAR_CELLS) + u32(cell.x);
    let first = hash(id);
    if unit(first) >= STAR_CHANCE {
        return vec3<f32>(0.0);
    }
    let second = hash(first);
    let third = hash(second);
    let jitter = vec2<f32>(unit(second), unit(second >> 16u));
    let at = cell + STAR_MARGIN + jitter * (1.0 - 2.0 * STAR_MARGIN);
    let star = tan((at / (0.5 * STAR_CELLS) - 1.0) * (PI / 4.0));
    // Within one face, the chord between the two unit directions is the angle between them.
    let apart = length(normalize(vec3<f32>(across, 1.0)) - normalize(vec3<f32>(star, 1.0)));
    let reach = STAR_REACH_PX * pixel_rad;
    let near = max(1.0 - (apart * apart) / (reach * reach), 0.0);
    // Most stars faint and a few bright; a few warm and a few blue.
    let brightness = pow(unit(first >> 16u), 6.0);
    let warmth = unit(third) - 0.5;
    let tint = vec3<f32>(1.0 + 0.3 * warmth, 1.0, 1.0 - 0.3 * warmth);
    return tint * (max(STAR_BRIGHTNESS * brightness - washed, 0.0) * near * near);
}

// Where the pixel looking `direction` falls on the moon's disc, in the disc's radii.
fn moon_offset(direction: vec3<f32>, facing: f32) -> vec3<f32> {
    return (direction - sky.toward_moon.xyz * facing) / MOON_RADIUS_RAD;
}

// How much of the pixel the disc covers, smoothed over one pixel.
fn moon_disc(facing: f32, direction: vec3<f32>, pixel_rad: f32) -> f32 {
    let out = length(moon_offset(direction, facing));
    return 1.0 - smoothstep(1.0 - pixel_rad / MOON_RADIUS_RAD, 1.0, out);
}

// The face as a sphere lit from `sky.moon_light`; its dark part is the sky without the stars, which
// the moon hides.
fn moon_face(behind: vec3<f32>, direction: vec3<f32>, facing: f32) -> vec3<f32> {
    let off = moon_offset(direction, facing);
    let depth = sqrt(max(1.0 - dot(off, off), 0.0));
    let normal = off - sky.toward_moon.xyz * depth;
    let lit = smoothstep(0.0, MOON_TERMINATOR, dot(normal, sky.moon_light.xyz));
    return mix(behind, seen(MOON_COLOUR * mix(MOON_LIMB, 1.0, depth)), lit);
}

@fragment
fn fragment(in: SkyOutput) -> @location(0) vec4<f32> {
    let near = view.world_from_clip * vec4<f32>(in.ndc, 1.0, 1.0);
    let direction = normalize(near.xyz / near.w - view.world_position.xyz);

    var low = sky.zenith.rgb;
    @if(DISTANCE_FOG) {
        low = seen(horizon(direction));
    }
    let up = zenith_share(direction);
    var colour = mix(low, sky.zenith.rgb, up);

    // The disc sets behind the horizon; its glow fades with the sun's height rather than at the
    // horizon line, which would draw an edge across the sky seen from above the reach.
    let above = smoothstep(-SUN_EDGE_RAD, 0.0, direction.y);
    let risen = smoothstep(-GLOW_SET, 0.0, sky.toward_sun.y);
    let facing = dot(direction, sky.toward_sun.xyz);
    let angle = acos(clamp(facing, -1.0, 1.0));
    let disc = above * (1.0 - smoothstep(SUN_RADIUS_RAD, SUN_RADIUS_RAD + SUN_EDGE_RAD, angle));
    let toward = max(facing, 0.0);
    let glow = risen * (GLOW_WIDE * pow(toward, GLOW_WIDE_POWER) + GLOW_TIGHT * pow(toward, GLOW_TIGHT_POWER));
    colour = mix(colour + seen(GLOW_COLOUR * glow), seen(SUN_COLOUR), disc);

    // The Cloud layer draws over all of this in the blended pass; see clouds.rs.
    let haze = 1.0 - exp(-max(direction.y, 0.0) * MOON_HAZE);
    if haze <= 0.0 {
        return vec4<f32>(colour, 1.0);
    }
    // One pixel's angle at the middle of the screen; a pixel at the edge spans less.
    let pixel_rad = 2.0 / (view.clip_from_view[1][1] * view.viewport.w);
    let moonward = dot(direction, sky.toward_moon.xyz);
    let fullness = 0.5 - 0.5 * dot(sky.moon_light.xyz, sky.toward_moon.xyz);
    if moonward > MOON_GLOW_FROM {
        let moon_glow = MOON_GLOW * fullness * pow(moonward, MOON_GLOW_POWER);
        colour += seen(MOON_COLOUR * moon_glow) * haze;
    }
    let behind_the_moon = colour;
    if sky.stars > 0.004 {
        let among_stars = (sky.stars_from_world * vec4<f32>(direction, 0.0)).xyz;
        let washed = MOON_WASH * fullness * smoothstep(0.0, 0.3, sky.toward_moon.y);
        colour += star_light(among_stars, pixel_rad, washed) * (sky.stars * up);
    }
    if moonward > cos(MOON_RADIUS_RAD) - 1e-4 {
        let face = moon_face(behind_the_moon, direction, moonward);
        colour = mix(colour, face, moon_disc(moonward, direction, pixel_rad) * haze);
    }
    return vec4<f32>(colour, 1.0);
}
