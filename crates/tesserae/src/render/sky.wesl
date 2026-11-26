// The sky gradient and the sun, behind everything; see render/sky.rs.

import bevy_pbr::render::mesh_view_bindings::{fog, lights, view};
@if(TONEMAP_IN_SHADER) import bevy_core_pipeline::tonemapping::tone_mapping;

// Mirrors SkyMaterial in sky.rs.
struct Sky {
    zenith: vec4<f32>,
    toward_sun: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> sky: Sky;

// How fast the horizon's colour gives way to the zenith's with height: half of
// the way by about 3 degrees, so even a view pitched down shows blue above a
// lighter band.
const HORIZON_FALLOFF: f32 = 12.0;

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

// The glow bevy_pbr's fog adds toward each directional light (its `apply_fog`), so the horizon
// glows as the fogged terrain at the reach does and the reach's edge stays hidden. No shadow:
// the reach is past the last cascade.
fn fog_glow(direction: vec3<f32>) -> vec3<f32> {
    if fog.directional_light_color.a <= 0.0 {
        return vec3<f32>(0.0);
    }
    var glow = vec3<f32>(0.0);
    for (var i: u32 = 0u; i < lights.n_directional_lights; i = i + 1u) {
        let light = lights.directional_lights[i];
        let toward = max(dot(direction, light.direction_to_light), 0.0);
        glow += pow(toward, fog.directional_light_exponent) * light.color.rgb * view.exposure;
    }
    return glow * fog.directional_light_color.rgb * fog.directional_light_color.a;
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

@fragment
fn fragment(in: SkyOutput) -> @location(0) vec4<f32> {
    let near = view.world_from_clip * vec4<f32>(in.ndc, 1.0, 1.0);
    let direction = normalize(near.xyz / near.w - view.world_position.xyz);

    var horizon = sky.zenith.rgb;
    @if(DISTANCE_FOG) {
        horizon = seen(fog.base_color.rgb + fog_glow(direction));
    }
    let up = 1.0 - exp(-max(direction.y, 0.0) * HORIZON_FALLOFF);
    var colour = mix(horizon, sky.zenith.rgb, up);

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
    return vec4<f32>(colour, 1.0);
}
