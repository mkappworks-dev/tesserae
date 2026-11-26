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
// The noise, baked over one period: red the coarse octaves, green the fine; see cloud_field.rs.
@group(constants::MATERIAL_BIND_GROUP) @binding(1)
var field_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2)
var field_sampler: sampler;

// The field's spread, measured over the period. With it the cover's threshold is placed by the
// logistic fit to the normal curve, so a cover of 0.3 leaves about 70% of the sky clear.
const FIELD_SIGMA: f32 = 0.133;
const LOGISTIC_FIT: f32 = 1.702;
// The warp bends the lattice's straight runs into billows: coarser than the field, and up to
// about two thirds of its coarsest cell.
const WARP_CELLS: f32 = 32.0;
const WARP_M: f32 = 200.0;
// The finest octaves fade out between these distances along the ray: further, a pixel and the
// spacing between samples are wider than their cells, and they shimmer.
const DETAIL_FROM_M: f32 = 600.0;
const DETAIL_TO_M: f32 = 2500.0;
// Samples further apart than the first step a far cloud's edge into combs; from the second the
// dither spreads them fully. Only far from the Eye: near, the grain shows as dots, and the samples
// are close enough.
const DITHER_FROM_M: f32 = 22.0;
const DITHER_TO_M: f32 = 37.5;
const DITHER_NEAR_M: f32 = 150.0;
const DITHER_FAR_M: f32 = 600.0;

// Samples along the ray inside the slab: one per this many metres of it, at least the first count
// for a sense of thickness and at most the second. Only rays near level reach the cap, a thin band
// round the horizon, which stays within the Cloud layer's 0.2 ms.
const SAMPLE_EVERY_M: f32 = 25.0;
const MIN_SAMPLES: u32 = 4u;
const MAX_SAMPLES: u32 = 16u;
// Past this much cloud a ray is opaque anyway, and the samples would spread too thin to see.
const MAX_MARCH_M: f32 = 600.0;
// Light lost per metre through the densest cloud: 50 m of it hides nearly all behind it.
const EXTINCTION_PER_M: f32 = 0.06;
// How far round the Eye the cloud it stands in fogs the view.
const IN_CLOUD_M: f32 = 60.0;
// How far past the cover's threshold the field climbs, in its spreads, before a column reaches
// the slab's top.
const BILLOW_SIGMAS: f32 = 1.0;
// How soft a cloud's flat base and its rounded top are, as shares of the thickness.
const SOFT_BASE: f32 = 0.08;
const SOFT_TOP: f32 = 0.15;

// The shares of the light and the sky a cloud's underside keeps against its sunlit top.
const UNDERSIDE_LIGHT: f32 = 0.5;
const UNDERSIDE_AMBIENT: f32 = 0.8;
// The silver round a cloud seen toward the light.
const FORWARD_SCATTER: f32 = 0.4;
const FORWARD_POWER: f32 = 8.0;

const INSIDE_SPAN: f32 = 16.0;

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
    let eye = view.world_position.xyz;
    let span = face_span(eye);
    let world = vec3<f32>(
        eye.x + in.position.x * span,
        clouds.slab.y + in.position.y * clouds.slab.w,
        eye.z + in.position.z * span,
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

const TAU: f32 = 6.2831853;

// The contribution of a lattice corner's gradient at `f`, from that corner; the lattice wrapped
// to `cells` so the noise repeats.
fn corner(cell: vec2<f32>, cells: f32, f: vec2<f32>) -> f32 {
    let wrapped = vec2<u32>(cell - cells * floor(cell / cells));
    let angle = f32(hash(wrapped.x + hash(wrapped.y)) & 0xffffu) / 65535.0 * TAU;
    return dot(vec2<f32>(cos(angle), sin(angle)), f);
}

// Gradient noise about 0 at `q`, a point in one period from 0 to 1 on each axis: no value sits on
// the lattice, so its rows do not show the way value noise's do.
fn gradient_noise(q: vec2<f32>, cells: f32) -> f32 {
    let p = q * cells;
    let i = floor(p);
    let f = p - i;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = corner(i, cells, f);
    let b = corner(i + vec2<f32>(1.0, 0.0), cells, f - vec2<f32>(1.0, 0.0));
    let c = corner(i + vec2<f32>(0.0, 1.0), cells, f - vec2<f32>(0.0, 1.0));
    let d = corner(i + vec2<f32>(1.0, 1.0), cells, f - vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// How far the field is bent at `m`, metres in the noise's own frame. Slow to vary, so one per
// pixel serves every sample along its ray.
fn warp(m: vec2<f32>) -> vec2<f32> {
    let q = fract(m / clouds.period_m);
    return WARP_M * vec2<f32>(
        gradient_noise(q, WARP_CELLS),
        gradient_noise(fract(q + vec2<f32>(0.5, 0.25)), WARP_CELLS),
    );
}

// The cloud field about 0 at `m`, metres in the noise's own frame, with `detail` from 0 to 1 of the
// finest octaves, which fade without moving the cover.
fn field(m: vec2<f32>, detail: f32) -> f32 {
    let octaves = textureSampleLevel(field_texture, field_sampler, fract(m / clouds.period_m), 0.0).rg;
    return (octaves.r + detail * octaves.g) * bitcast<f32>(constants::CLOUD_FIELD_RANGE);
}

// Where the field crosses into cloud for the cover.
fn threshold() -> f32 {
    let cover = clamp(clouds.field.w, 0.001, 0.999);
    return FIELD_SIGMA * log((1.0 - cover) / cover) / LOGISTIC_FIT;
}

// How dense the cloud is where the field reads `n`, at `h` from 0 (the base) to 1 (the top): every
// cloud sits on the slab's base, and one the field clears the threshold by more stands taller.
fn density(n: f32, h: f32, at: f32) -> f32 {
    let over = clamp((n - at) / (BILLOW_SIGMAS * FIELD_SIGMA), 0.0, 1.0);
    let top = sqrt(over);
    return smoothstep(0.0, SOFT_BASE, h) * smoothstep(0.0, SOFT_TOP, top - h);
}

// A per-pixel offset for the samples, so their spacing shows as fine grain, not bands. Hashed
// rather than interleaved gradient noise: with no temporal smoothing that one's diagonals show.
fn dither(pixel: vec2<f32>) -> f32 {
    let p = vec2<u32>(pixel);
    return f32(hash(p.x + hash(p.y)) & 0xffffu) / 65536.0;
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

// How far round the Eye the faces reach. From inside, a ray near level meets neither face within
// the reach: spread far beyond it, the gap at the horizon is under a pixel.
fn face_span(eye: vec3<f32>) -> f32 {
    let inside = eye.y > clouds.slab.y && eye.y < clouds.slab.y + clouds.slab.w;
    return select(1.0, INSIDE_SPAN, inside) * clouds.field.z;
}

// A cloud's colour, linear, `along` metres down `direction` at `height` from 0 (its base) to 1.
fn shade(direction: vec3<f32>, height: f32, along: f32) -> vec3<f32> {
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
    return colour.rgb;
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
    let samples = clamp(u32(ceil(max(leave - enter, 0.0) / SAMPLE_EVERY_M)), MIN_SAMPLES, MAX_SAMPLES);
    let spacing = max(leave - enter, 0.0) / f32(samples);

    let span = face_span(eye);
    let across = length(in.world.xz - eye.xz);
    let edge = 1.0 - smoothstep(EDGE_FADE_FROM * span, span, across);

    // Relative to the Eye the origin was taken under, so no large coordinate enters the noise.
    let under_eye = clouds.field.xy + (eye.xz - clouds.slab.xz);
    let at = threshold();
    let bend = warp(under_eye + direction.xz * 0.5 * (enter + leave));
    let spread = smoothstep(DITHER_FROM_M, DITHER_TO_M, spacing)
        * smoothstep(DITHER_NEAR_M, DITHER_FAR_M, enter);
    let jitter = 0.5 + spread * (dither(in.clip.xy) - 0.5);
    let detail = 1.0 - smoothstep(DETAIL_FROM_M, DETAIL_TO_M, enter);
    // Metres of the densest cloud the ray crosses, and what it sees weighted by them.
    var depth_m = 0.0;
    var height = 0.0;
    var along = 0.0;
    for (var i: u32 = 0u; i < samples; i = i + 1u) {
        let t = enter + (f32(i) + jitter) * spacing;
        let h = clamp((eye.y + direction.y * t - base) / thickness, 0.0, 1.0);
        let m = density(field(under_eye + bend + direction.xz * t, detail), h, at) * spacing * edge;
        depth_m += m;
        height += m * h;
        along += m * t;
    }
    // Inside the slab the cloud round the Eye fogs every ray, which samples spread along a long
    // ray would step over. It stands in front of them, so it is not fogged by their distance.
    var near_m = 0.0;
    let eye_h = clamp((eye.y - base) / thickness, 0.0, 1.0);
    if enter <= 0.0 {
        near_m = density(field(under_eye + warp(under_eye), 1.0), eye_h, at) * min(leave, IN_CLOUD_M);
    }
    if depth_m + near_m <= 0.0 {
        discard;
    }
    let far_alpha = 1.0 - exp(-EXTINCTION_PER_M * depth_m);
    let near_alpha = 1.0 - exp(-EXTINCTION_PER_M * near_m);
    var far = vec3<f32>(0.0);
    if depth_m > 0.0 {
        far = shade(direction, height / depth_m, along / depth_m);
    }
    var near = vec3<f32>(0.0);
    if near_m > 0.0 {
        near = shade(direction, eye_h, 0.5 * min(leave, IN_CLOUD_M));
    }
    let alpha = 1.0 - (1.0 - near_alpha) * (1.0 - far_alpha);
    var colour = vec4<f32>(
        (near * near_alpha + far * far_alpha * (1.0 - near_alpha)) / alpha,
        1.0,
    );
    @if(TONEMAP_IN_SHADER) {
        colour = tone_mapping(colour, view.color_grading);
    }
    return vec4<f32>(colour.rgb, alpha);
}
