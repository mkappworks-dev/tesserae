// The sky gradient and the sun, behind everything; see render/sky.rs.

import bevy_pbr::render::mesh_view_bindings::{fog, view};
@if(TONEMAP_IN_SHADER) import bevy_core_pipeline::tonemapping::tone_mapping;

// Mirrors SkyMaterial in sky.rs.
struct Sky {
    zenith: vec4<f32>,
    toward_sun: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> sky: Sky;

// How fast the horizon's colour gives way to the zenith's with height: half of
// the way by about 6 degrees, so even a view pitched down shows blue above a
// lighter band.
const HORIZON_FALLOFF: f32 = 6.0;

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

@fragment
fn fragment(in: SkyOutput) -> @location(0) vec4<f32> {
    let near = view.world_from_clip * vec4<f32>(in.ndc, 1.0, 1.0);
    let direction = normalize(near.xyz / near.w - view.world_position.xyz);

    var horizon = sky.zenith.rgb;
    @if(DISTANCE_FOG) {
        horizon = fog.base_color.rgb;
    }
    let up = 1.0 - exp(-max(direction.y, 0.0) * HORIZON_FALLOFF);
    var colour = mix(horizon, sky.zenith.rgb, up);

    let facing = dot(direction, sky.toward_sun.xyz);
    let angle = acos(clamp(facing, -1.0, 1.0));
    let disc = 1.0 - smoothstep(SUN_RADIUS_RAD, SUN_RADIUS_RAD + SUN_EDGE_RAD, angle);
    let toward = max(facing, 0.0);
    let glow = GLOW_WIDE * pow(toward, GLOW_WIDE_POWER) + GLOW_TIGHT * pow(toward, GLOW_TIGHT_POWER);
    colour = mix(colour + GLOW_COLOUR * glow, SUN_COLOUR, disc);

    var out = vec4<f32>(colour, 1.0);
    @if(TONEMAP_IN_SHADER) {
        out = tone_mapping(out, view.color_grading);
    }
    return out;
}
