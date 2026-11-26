// The Cloud layer: one face of the slab, shaded from a few samples of 2D noise along the view ray
// inside it; see render/clouds.rs.

import bevy_pbr::render::mesh_view_bindings::{fog, lights, view};
import bevy_pbr::render::fog::exponential_squared_fog;
@if(TONEMAP_IN_SHADER) import bevy_core_pipeline::tonemapping::tone_mapping;

// Mirrors CloudMaterial in clouds.rs.
struct Clouds {
    slab: vec4<f32>,
    field: vec4<f32>,
    light: vec4<f32>,
    ambient: vec4<f32>,
    toward_light: vec4<f32>,
    period_m: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> clouds: Clouds;

// Lattice cells across one noise period in the coarse octave: 640 m each over the 20 480 m
// period. The fine octave has four times as many. Whole numbers, so both repeat with the period.
const CELLS: f32 = 32.0;
const FINE: f32 = 4.0;
const FINE_SHARE: f32 = 0.35;

// Samples along the ray inside the slab: enough for a sense of thickness, few enough to stay
// within the Cloud layer's 0.2 ms.
const SAMPLES: u32 = 4u;
// Past this much cloud a ray is opaque anyway, and the samples would spread too thin to see.
const MAX_MARCH_M: f32 = 1500.0;
// Light lost per metre through the densest cloud: 30 m of it hides nearly all behind it.
const EXTINCTION_PER_M: f32 = 0.1;
// How far past the cover's threshold the noise climbs before a column fills the slab's thickness.
const BILLOW: f32 = 0.3;
// How soft a cloud's top and bottom are, as a share of the thickness.
const SOFT_EDGE: f32 = 0.15;

// The shares of the light and the sky a cloud's underside keeps against its sunlit top.
const UNDERSIDE_LIGHT: f32 = 0.5;
const UNDERSIDE_AMBIENT: f32 = 0.8;
// The silver round a cloud seen toward the light.
const FORWARD_SCATTER: f32 = 0.4;
const FORWARD_POWER: f32 = 8.0;

// The slab fades out over the last quarter of the reach, so its square edge never shows.
const EDGE_FADE_FROM: f32 = 0.75;

// The ray never runs exactly along a face it is drawn on, but can come close enough to overflow.
const MIN_RISE: f32 = 1e-5;

struct Vertex {
    @location(0) position: vec3<f32>,
}

struct CloudOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
}

// The face is a square from -1 to 1 at y 0 (the base) or 1 (the top), centred under the camera
// and scaled to the reach; every transform is ignored.
@vertex
fn vertex(in: Vertex) -> CloudOutput {
    let reach = clouds.field.z;
    let world = vec3<f32>(
        view.world_position.x + in.position.x * reach,
        clouds.slab.y + in.position.y * clouds.slab.w,
        view.world_position.z + in.position.z * reach,
    );
    var out: CloudOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.world = world;
    return out;
}

fn hash(x: u32) -> u32 {
    // PCG, after Jarzynski and Olano's "Hash Functions for GPU Rendering".
    let state = x * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// The value at a lattice corner, the lattice wrapped to `cells` so the noise repeats.
fn corner(cell: vec2<f32>, cells: f32) -> f32 {
    let wrapped = vec2<u32>(cell - cells * floor(cell / cells));
    return f32(hash(wrapped.x + hash(wrapped.y)) & 0xffffu) / 65535.0;
}

// Value noise from 0 to 1 at `q`, a point in one period from 0 to 1 on each axis.
fn value_noise(q: vec2<f32>, cells: f32) -> f32 {
    let p = q * cells;
    let i = floor(p);
    let f = p - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = corner(i, cells);
    let b = corner(i + vec2<f32>(1.0, 0.0), cells);
    let c = corner(i + vec2<f32>(0.0, 1.0), cells);
    let d = corner(i + vec2<f32>(1.0, 1.0), cells);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// The cloud field at `m`, metres in the noise's own frame.
fn field(m: vec2<f32>) -> f32 {
    let q = fract(m / clouds.period_m);
    return mix(value_noise(q, CELLS), value_noise(q, CELLS * FINE), FINE_SHARE);
}

// How dense the cloud is where the field reads `n`, at `h` from 0 (the base) to 1 (the top): a
// column the noise clears the cover's threshold by more stands taller about the slab's middle.
fn density(n: f32, h: f32) -> f32 {
    let cover = clouds.field.w;
    let over = clamp((n - (1.0 - cover)) / BILLOW, 0.0, 1.0);
    let half_height = 0.5 * sqrt(over);
    return smoothstep(0.0, SOFT_EDGE, half_height - abs(h - 0.5));
}

// The glow bevy_pbr's fog adds toward each directional light, as `sky.wesl`'s `fog_glow`, so a
// cloud at the reach fades into the same horizon the sky draws.
fn scattering(direction: vec3<f32>) -> vec3<f32> {
    var glow = vec3<f32>(0.0);
    for (var i: u32 = 0u; i < lights.n_directional_lights; i = i + 1u) {
        let light = lights.directional_lights[i];
        let toward = max(dot(direction, light.direction_to_light), 0.0);
        glow += pow(toward, fog.directional_light_exponent) * light.color.rgb * view.exposure;
    }
    return glow;
}

@fragment
fn fragment(in: CloudOutput) -> @location(0) vec4<f32> {
    let eye = view.world_position.xyz;
    let direction = normalize(in.world - eye);
    let base = clouds.slab.y;
    let thickness = clouds.slab.w;
    let least = select(MIN_RISE, -MIN_RISE, direction.y < 0.0);
    let rise = select(direction.y, least, abs(direction.y) < MIN_RISE);
    let to_base = (base - eye.y) / rise;
    let to_top = (base + thickness - eye.y) / rise;
    // From where the ray enters the slab, or the Eye inside it, to where it leaves.
    let enter = max(min(to_base, to_top), 0.0);
    let leave = min(max(to_base, to_top), enter + MAX_MARCH_M);
    let spacing = max(leave - enter, 0.0) / f32(SAMPLES);

    // Relative to the Eye the origin was taken under, so no large coordinate enters the noise.
    let under_eye = clouds.field.xy + (eye.xz - clouds.slab.xz);
    var amount = 0.0;
    var height = 0.0;
    var along = 0.0;
    for (var i: u32 = 0u; i < SAMPLES; i = i + 1u) {
        let t = enter + (f32(i) + 0.5) * spacing;
        let h = clamp((eye.y + direction.y * t - base) / thickness, 0.0, 1.0);
        let d = density(field(under_eye + direction.xz * t), h);
        amount += d;
        height += d * h;
        along += d * t;
    }
    if amount <= 0.0 {
        discard;
    }
    height /= amount;
    along /= amount;
    let alpha = 1.0 - exp(-EXTINCTION_PER_M * amount * spacing);

    let toward = max(dot(direction, clouds.toward_light.xyz), 0.0);
    let sunlit = mix(UNDERSIDE_LIGHT, 1.0, height) + FORWARD_SCATTER * pow(toward, FORWARD_POWER);
    var colour = vec4<f32>(
        clouds.light.rgb * sunlit + clouds.ambient.rgb * mix(UNDERSIDE_AMBIENT, 1.0, height),
        1.0,
    );
    // fog.rs always sets ExponentialSquared; fogged at the cloud seen, not at the face drawn.
    @if(DISTANCE_FOG) {
        colour = exponential_squared_fog(fog, colour, along, scattering(direction));
    }
    @if(TONEMAP_IN_SHADER) {
        colour = tone_mapping(colour, view.color_grading);
    }

    let reach = clouds.field.z;
    let across = length(in.world.xz - eye.xz);
    let edge = 1.0 - smoothstep(EDGE_FADE_FROM * reach, reach, across);
    return vec4<f32>(colour.rgb, alpha * edge);
}
