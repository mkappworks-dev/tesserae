// The sky's colour along a ray, before tonemapping: what sky.wesl draws behind everything and what
// water reflects.

import bevy_pbr::render::mesh_view_bindings::{fog, lights, view};

// How fast the horizon's colour gives way to the zenith's with height: half of
// the way by about 3 degrees, so even a view pitched down shows blue above a
// lighter band.
const HORIZON_FALLOFF: f32 = 12.0;

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

// The fogged terrain's colour at the reach, seen along `direction`.
fn horizon(direction: vec3<f32>) -> vec3<f32> {
    return fog.base_color.rgb + fog_glow(direction);
}

// The share of the zenith's colour in the sky along `direction`.
fn zenith_share(direction: vec3<f32>) -> f32 {
    return 1.0 - exp(-max(direction.y, 0.0) * HORIZON_FALLOFF);
}

// The zenith is the sky colour as authored, which sky.wesl shows untonemapped; here it stands in
// as radiance, so a reflection of it greys a little through the tonemapper, as the clear colour did.
fn sky_before_tonemap(direction: vec3<f32>, zenith: vec3<f32>) -> vec3<f32> {
    var low = zenith;
    @if(DISTANCE_FOG) {
        low = horizon(direction);
    }
    return mix(low, zenith, zenith_share(direction));
}
