// The water path of the chunk shader; see render/water.rs.

import bevy_pbr::render::{
    mesh_view_bindings::{globals, lights, view},
    pbr_lighting::{EnvBRDFApprox, F_AB},
    pbr_types::PbrInput,
};
import super::{
    chunk_vertex::{KEY_AXIS_SHIFT, KEY_CELL_MASK, KEY_DEPTH_SHIFT, KEY_V_SHIFT, quads},
    sky_colour::sky_before_tonemap,
};

// water::table's rows, three per palette slot.
@group(constants::MATERIAL_BIND_GROUP) @binding(107)
var<uniform> water_table: array<vec4<f32>, constants::CHUNK_WATER_ROWS>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108)
var water_normals: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(109)
var water_normals_sampler: sampler;

// Mirrors WaterView in chunk_material.rs.
struct WaterView {
    zenith: vec4<f32>,
    flow_fade_m: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(110)
var<uniform> water_view: WaterView;
@group(constants::MATERIAL_BIND_GROUP) @binding(111)
var water_foam: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112)
var water_foam_sampler: sampler;

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

// A Chunk is a whole number of cells from the origin and 1024 is a multiple of its side, so a
// Voxel key's cells, wrapped to the Chunk, are the Voxel's place in it.
const CHUNK_CELL_MASK: u32 = constants::CHUNK_CELLS - 1u;

// The fragment's place in its Chunk's water columns, in Voxels along x and z: the quad's first
// Voxel plus how far across the quad the fragment is. Not stepped Voxel by Voxel as `voxel_at`
// does: a fragment at a quad's far edge would step into the next Chunk and wrap to this one's
// near side.
fn column_point(key: u32, uv: vec2<f32>) -> vec2<f32> {
    let along_u = f32(key & KEY_CELL_MASK & CHUNK_CELL_MASK) + uv.x;
    let along_v = f32((key >> KEY_V_SHIFT) & KEY_CELL_MASK & CHUNK_CELL_MASK) + uv.y;
    let behind = f32((key >> KEY_DEPTH_SHIFT) & KEY_CELL_MASK & CHUNK_CELL_MASK) + 0.5;
    // A face's u and v follow its normal's axis: y and z for x, z and x for y, x and y for z.
    switch key >> KEY_AXIS_SHIFT {
        case 0u: {
            return vec2<f32>(behind, along_v);
        }
        case 1u: {
            return vec2<f32>(along_v, along_u);
        }
        default: {
            return vec2<f32>(along_u, behind);
        }
    }
}

// `tesserae_core::water_columns::WaterColumn`'s depth, at column `x`, `z` of the apron'd grid.
fn column_depth_m(water: u32, column: vec2<i32>) -> f32 {
    let side = i32(constants::CHUNK_WATER_SIDE);
    let word = quads[water + u32((column.y + 1) * side + column.x + 1)];
    return f32((word >> 16u) & 0xffu) * bitcast<f32>(constants::CHUNK_WATER_DEPTH_STEP_M);
}

// Depth filtered between column centres. The apron holds the neighbours' columns, so two Chunks
// of one lake meet with no step. A dry column stands as deep below as the wet ones beside it are
// above, so the depth runs out to nothing at the waterline and not half a column short of it.
fn water_depth_m(water: u32, point: vec2<f32>) -> f32 {
    let q = point - 0.5;
    let last = i32(constants::CHUNK_CELLS) - 1;
    let i = clamp(vec2<i32>(floor(q)), vec2<i32>(-1), vec2<i32>(last));
    let f = clamp(q - vec2<f32>(i), vec2<f32>(0.0), vec2<f32>(1.0));
    var depth = vec4<f32>(
        column_depth_m(water, i),
        column_depth_m(water, i + vec2<i32>(1, 0)),
        column_depth_m(water, i + vec2<i32>(0, 1)),
        column_depth_m(water, i + vec2<i32>(1, 1)),
    );
    let wet = vec4<f32>(depth > vec4<f32>(0.0));
    let wet_mean = dot(depth, wet) / max(dot(wet, vec4<f32>(1.0)), 1.0);
    depth = select(vec4<f32>(-wet_mean), depth, depth > vec4<f32>(0.0));
    let filtered = mix(mix(depth.x, depth.y, f.x), mix(depth.z, depth.w, f.x), f.y);
    return max(filtered, 0.0);
}

// `WaterColumn`'s Flow at column `x`, `z`, in m/s along x and z: two signed bytes.
fn column_flow(water: u32, column: vec2<i32>) -> vec2<f32> {
    let side = i32(constants::CHUNK_WATER_SIDE);
    let word = quads[water + u32((column.y + 1) * side + column.x + 1)];
    let steps = vec2<i32>(bitcast<i32>(word << 24u) >> 24u, bitcast<i32>(word << 16u) >> 24u);
    return vec2<f32>(steps) * bitcast<f32>(constants::CHUNK_WATER_FLOW_STEP_M_S);
}

// Flow filtered between column centres as depth is, faded to calm toward the edge of level 0,
// so it meets coarse water, which carries none, without a step.
fn water_flow(water: u32, point: vec2<f32>, world_position: vec3<f32>) -> vec2<f32> {
    let q = point - 0.5;
    let last = i32(constants::CHUNK_CELLS) - 1;
    let i = clamp(vec2<i32>(floor(q)), vec2<i32>(-1), vec2<i32>(last));
    let f = clamp(q - vec2<f32>(i), vec2<f32>(0.0), vec2<f32>(1.0));
    let low = mix(column_flow(water, i), column_flow(water, i + vec2<i32>(1, 0)), f.x);
    let high = mix(
        column_flow(water, i + vec2<i32>(0, 1)),
        column_flow(water, i + vec2<i32>(1, 1)),
        f.x,
    );
    let distance = length(world_position - view.world_position.xyz);
    let fade = 1.0 - smoothstep(water_view.flow_fade_m.x, water_view.flow_fade_m.y, distance);
    return mix(low, high, f.y) * fade;
}

// Scale of the slow noise that staggers the flow cycle from place to place, so the cross-fade's
// pulse never sweeps a whole river at once.
const FLOW_STAGGER_M: f32 = 23.0;

// How many one-second flow cycles have passed at `xz`. A whole number of cycles per period, so
// it steps by a whole number where the clock wraps.
fn flow_cycle(xz: vec2<f32>) -> f32 {
    let stagger = textureSampleLevel(water_foam, water_foam_sampler, xz / FLOW_STAGGER_M, 3.0).r;
    return clock() * (f32(constants::CHUNK_FLOW_CYCLES) / f32(constants::CHUNK_CLOCK_PERIOD_S)) + stagger;
}

// `water::FLOW_JUMP`: where each new cycle starts the texture, so the surface does not repeat
// every half cycle.
fn flow_jump() -> vec2<f32> {
    return vec2<f32>(bitcast<f32>(constants::CHUNK_FLOW_JUMP_X), bitcast<f32>(constants::CHUNK_FLOW_JUMP_Z));
}

// Two samples of a texture dragged along `flow` half a cycle apart, each weighted by how near the
// middle of its drag it is, so neither is seen at the end of its drag where it stretches most.
// `flow` is in the texture's repeats per cycle. Cross-faded noise loses contrast midway; the
// weights' length puts it back.
struct FlowTaps {
    first: vec2<f32>,
    second: vec2<f32>,
    weights: vec2<f32>,
}

fn flow_taps(uv: vec2<f32>, flow: vec2<f32>, cycle: f32) -> FlowTaps {
    let first = fract(cycle);
    let second = fract(cycle + 0.5);
    let first_weight = 1.0 - abs(2.0 * first - 1.0);
    let weights = vec2<f32>(first_weight, 1.0 - first_weight) / length(vec2<f32>(first_weight, 1.0 - first_weight));
    return FlowTaps(
        uv - flow * (first - 0.5) + floor(cycle) * flow_jump(),
        uv - flow * (second - 0.5) + floor(cycle + 0.5) * flow_jump() + 0.5,
        weights,
    );
}

// Shallower than this, the water foams where the mask says.
const FOAM_DEPTH_M: f32 = 0.6;
// One repeat of the foam mask.
const FOAM_TILE_M: f32 = 3.0;
// How soft the foam's edge is, in the mask's units.
const FOAM_EDGE: f32 = 0.12;

// How much of the surface is foam at `depth_m`: a band along the shore whose inner edge the mask
// breaks into streaks.
// White water: from this speed the surface starts to foam, and at the next it is all foam where
// the mask allows. The Generator's pools run near 3 m/s and its steepest reaches at its 4 m/s
// cap, so white water is kept to those.
const WHITE_FROM_M_S: f32 = 3.0;
const WHITE_AT_M_S: f32 = 3.9;

// How much of the surface is foam: a band along the shore, and white water where it runs fast,
// each broken up by the mask, which runs with the Flow.
fn water_foam_share(
    xz: vec2<f32>,
    gradient: WaterGradient,
    depth_m: f32,
    flow: vec2<f32>,
    cycle: f32,
) -> f32 {
    let shore = 1.0 - smoothstep(0.03, FOAM_DEPTH_M, depth_m);
    let white = smoothstep(WHITE_FROM_M_S, WHITE_AT_M_S, length(flow));
    let band = max(shore, white);
    if band <= 0.0 {
        return 0.0;
    }
    let ddx = gradient.ddx / FOAM_TILE_M;
    let ddy = gradient.ddy / FOAM_TILE_M;
    let taps = flow_taps(xz / FOAM_TILE_M, flow / FOAM_TILE_M, cycle);
    let mask = dot(
        vec2<f32>(
            textureSampleGrad(water_foam, water_foam_sampler, taps.first, ddx, ddy).r,
            textureSampleGrad(water_foam, water_foam_sampler, taps.second, ddx, ddy).r,
        ),
        taps.weights,
    ) / (taps.weights.x + taps.weights.y);
    return smoothstep(1.0 - band, 1.0 - band + FOAM_EDGE, mask);
}

// The body of water under a fragment, premultiplied: what it sends toward the eye as albedo, and
// how much of what lies behind it it hides.
struct WaterBody {
    albedo: vec3<f32>,
    alpha: f32,
}

// Each channel keeps exp(-absorption x path) of the floor, the path being the depth over the
// cosine of the view; what is lost is replaced by the scatter tint. The floor is blended by one
// alpha, so alpha follows the mean of the three.
fn water_body(slot: u32, depth_m: f32, cos_view: f32, foam: f32) -> WaterBody {
    let absorption = water_table[slot * 3u].xyz;
    let scatter = water_table[slot * 3u + 1u].xyz;
    let path = depth_m / max(cos_view, 0.05);
    let lost = vec3<f32>(1.0) - exp(-absorption * path);
    let body = WaterBody(scatter * lost, (lost.x + lost.y + lost.z) / 3.0);
    let foam_colour = water_table[slot * 3u + 2u].xyz;
    return WaterBody(mix(body.albedo, foam_colour, foam), mix(body.alpha, 1.0, foam));
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

// Below this speed, in m/s, a layer is not flow-mapped: one sample, as still water.
const STILL_M_S: f32 = 0.02;

// `flow` in m/s along the layer's own axes.
fn ripple_layer(
    xz: vec2<f32>,
    gradient: WaterGradient,
    tile_m: f32,
    tiles_per_period: vec2<f32>,
    strength: f32,
    flow: vec2<f32>,
    cycle: f32,
) -> Ripples {
    // A whole number of repeats per period, so this is continuous where `globals.time` wraps.
    let drift = clock() * (tiles_per_period / f32(constants::CHUNK_CLOCK_PERIOD_S));
    let uv = xz / tile_m + drift;
    let ddx = gradient.ddx / tile_m;
    let ddy = gradient.ddy / tile_m;
    // How many of the layer's shortest crests one pixel spans.
    let crests = max(length(ddx), length(ddy)) * MOST_CRESTS;
    let lost = smoothstep(0.25, 2.0, crests);
    var slope: vec2<f32>;
    if length(flow) < STILL_M_S {
        slope = textureSampleGrad(water_normals, water_normals_sampler, uv, ddx, ddy).rg;
    } else {
        // One cycle is one second, so Flow in m/s is the drag in metres a cycle.
        let taps = flow_taps(uv, flow / tile_m, cycle);
        slope = textureSampleGrad(water_normals, water_normals_sampler, taps.first, ddx, ddy).rg * taps.weights.x
            + textureSampleGrad(water_normals, water_normals_sampler, taps.second, ddx, ddy).rg * taps.weights.y;
    }
    slope *= strength;
    // A sine's slope variance is half its peak squared.
    return Ripples(slope * (1.0 - lost), 0.5 * strength * strength * lost);
}

// How much rougher running water's ripples are, per m/s, so steep reaches read as rapids.
const FLOW_STRENGTH_PER_M_S: f32 = 0.35;

struct WaterSurface {
    normal: vec3<f32>,
    perceptual_roughness: f32,
}

// `normal` faces the viewer, as bevy_pbr's double-sided flip leaves it. Ripples move only the
// tops and undersides; a wall of water keeps its face's normal.
// `calm` scales the ripples: 0 flat, 1 full. `flow` drags them, in m/s along x and z.
fn water_surface(
    slot: u32,
    world_position: vec3<f32>,
    gradient: WaterGradient,
    normal: vec3<f32>,
    calm: f32,
    flow: vec2<f32>,
    cycle: f32,
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
        flow,
        cycle,
    );
    let b = ripple_layer(
        // Turned against the first, so the two layers' texels never line up.
        vec2<f32>(world_position.z, -world_position.x),
        WaterGradient(vec2<f32>(gradient.ddx.y, -gradient.ddx.x), vec2<f32>(gradient.ddy.y, -gradient.ddy.x)),
        bitcast<f32>(constants::CHUNK_RIPPLE1_TILE_M),
        vec2<f32>(f32(constants::CHUNK_RIPPLE1_X), f32(constants::CHUNK_RIPPLE1_Z)),
        bitcast<f32>(constants::CHUNK_RIPPLE1_STRENGTH),
        vec2<f32>(flow.y, -flow.x),
        cycle,
    );
    let turned_back = vec2<f32>(-b.slope.y, b.slope.x);
    let slope = (a.slope + turned_back) * calm * (1.0 + FLOW_STRENGTH_PER_M_S * length(flow));
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
    return sky_before_tonemap(normalize(vec3<f32>(ray.x, abs(ray.y), ray.z)), water_view.zenith.rgb);
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
