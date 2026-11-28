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
    wind: vec4<f32>,
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

fn column_word(water: u32, column: vec2<i32>) -> u32 {
    let side = i32(constants::CHUNK_WATER_SIDE);
    return quads[water + u32((column.y + 1) * side + column.x + 1)];
}

// `tesserae_core::water_columns::WaterColumn`'s depth, at column `x`, `z` of the apron'd grid.
fn column_depth_m(water: u32, column: vec2<i32>) -> f32 {
    let word = column_word(water, column);
    return f32((word >> 16u) & 0xffu) * bitcast<f32>(constants::CHUNK_WATER_DEPTH_STEP_M);
}

fn word_depth_m(word: u32) -> f32 {
    return f32((word >> 16u) & 0xffu) * bitcast<f32>(constants::CHUNK_WATER_DEPTH_STEP_M);
}

fn word_flag(word: u32, flag: u32) -> f32 {
    return select(0.0, 1.0, ((word >> 24u) & flag) != 0u);
}

// Flow in m/s along x and z: two signed bytes.
fn word_flow(word: u32) -> vec2<f32> {
    let steps = vec2<i32>(bitcast<i32>(word << 24u) >> 24u, bitcast<i32>(word << 16u) >> 24u);
    return vec2<f32>(steps) * bitcast<f32>(constants::CHUNK_WATER_FLOW_STEP_M_S);
}

// The water under a fragment, filtered between column centres: its depth, how the depth slopes
// in metres per metre along x and z, how much of it is the sea, how much a Fall lands in, and its
// Flow, faded to calm toward the edge of level 0 so it meets coarse water, which carries none,
// without a step.
struct WaterShape {
    depth_m: f32,
    slope: vec2<f32>,
    sea: f32,
    landing: f32,
    flow: vec2<f32>,
    // How much silt the water carries, 0 for still water and the sea, from the Flow before it is
    // calmed with distance, so a river keeps its colour out to level 0's edge.
    silt: f32,
}

// Four column values, x then z, at `f` between their centres.
fn bilinear(v: vec4<f32>, f: vec2<f32>) -> f32 {
    return mix(mix(v.x, v.y, f.x), mix(v.z, v.w, f.x), f.y);
}

// Each of the four columns around the fragment is read once. The apron holds the neighbours'
// columns, so two Chunks of one lake meet with no step. A dry column stands as deep below as the
// wet ones beside it are above, so the depth runs out to nothing at the waterline and not half a
// column short of it. `level` is the layer's LOD level: its cells are 2^level Voxels across.
fn water_shape(water: u32, point: vec2<f32>, level: u32, world_position: vec3<f32>) -> WaterShape {
    let q = point - 0.5;
    let last = i32(constants::CHUNK_CELLS) - 1;
    let i = clamp(vec2<i32>(floor(q)), vec2<i32>(-1), vec2<i32>(last));
    let f = clamp(q - vec2<f32>(i), vec2<f32>(0.0), vec2<f32>(1.0));
    let words = vec4<u32>(
        column_word(water, i),
        column_word(water, i + vec2<i32>(1, 0)),
        column_word(water, i + vec2<i32>(0, 1)),
        column_word(water, i + vec2<i32>(1, 1)),
    );
    var depth = vec4<f32>(0.0);
    var sea = vec4<f32>(0.0);
    var landing = vec4<f32>(0.0);
    var flow_x = vec4<f32>(0.0);
    var flow_z = vec4<f32>(0.0);
    var silt = vec4<f32>(0.0);
    for (var c = 0u; c < 4u; c++) {
        depth[c] = word_depth_m(words[c]);
        sea[c] = word_flag(words[c], constants::CHUNK_WATER_SEA);
        landing[c] = word_flag(words[c], constants::CHUNK_WATER_FOAM);
        let flow = word_flow(words[c]);
        flow_x[c] = flow.x;
        flow_z[c] = flow.y;
        silt[c] = smoothstep(SILT_FROM_M_S, SILT_AT_M_S, length(flow)) * (1.0 - sea[c]);
    }
    let wet = vec4<f32>(depth > vec4<f32>(0.0));
    let wet_mean = dot(depth, wet) / max(dot(wet, vec4<f32>(1.0)), 1.0);
    depth = select(vec4<f32>(-wet_mean), depth, depth > vec4<f32>(0.0));
    let per_voxel = vec2<f32>(
        mix(depth.y - depth.x, depth.w - depth.z, f.y),
        mix(depth.z - depth.x, depth.w - depth.y, f.x),
    );
    let cell_m = water_view.flow_fade_m.z * exp2(f32(level));
    let distance = length(world_position - view.world_position.xyz);
    let calm = 1.0 - smoothstep(water_view.flow_fade_m.x, water_view.flow_fade_m.y, distance);
    return WaterShape(
        max(bilinear(depth, f), 0.0),
        per_voxel / cell_m,
        bilinear(sea, f),
        bilinear(landing, f),
        vec2<f32>(bilinear(flow_x, f), bilinear(flow_z, f)) * calm,
        bilinear(silt, f),
    );
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

// Shallower than this, the sea foams at its shore where the mask says.
const FOAM_DEPTH_M: f32 = 0.6;
// The share of the surface the shore band, a breaker or a Fall's landing foams at most: below 1,
// so the mask still breaks it into lace and it never reads as a solid white patch.
const SHORE_FOAM_MOST: f32 = 0.6;
// One repeat of the foam mask.
const FOAM_TILE_M: f32 = 3.0;
// How soft the foam's edge is, in the mask's units.
const FOAM_EDGE: f32 = 0.12;

// White water: from this speed the surface starts to foam, and at the next it is all foam where
// the mask allows. The Generator's pools run near 3 m/s and its steepest reaches at its 4 m/s
// cap, so white water is kept to those.
const WHITE_FROM_M_S: f32 = 3.0;
const WHITE_AT_M_S: f32 = 3.9;

// How much of the surface is foam: a band along the sea's shore, white water where it runs fast,
// and `breaking`, the sea's breakers or a Fall's landing, each broken up by the mask, which runs
// with the Flow. Still water never foams at its edge: only the sea's swash churns a shore, and a
// shallow pond or a lake's margin is as clear as its middle.
fn water_foam_share(
    xz: vec2<f32>,
    gradient: WaterGradient,
    depth_m: f32,
    sea: f32,
    flow: vec2<f32>,
    cycle: f32,
    breaking: f32,
) -> f32 {
    let shore = (1.0 - smoothstep(0.03, FOAM_DEPTH_M, depth_m)) * sea * SHORE_FOAM_MOST;
    let white = smoothstep(WHITE_FROM_M_S, WHITE_AT_M_S, length(flow));
    let band = max(max(shore, white), breaking * SHORE_FOAM_MOST);
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

// A wall of water is falling water: a Fall's sheet, a Settlement's waterfall, a poured block's
// side. Streaks of the foam mask, stretched down the wall, run down it at this speed.
const FALL_M_S: f32 = 6.0;
const FALL_ACROSS_M: f32 = 0.8;
const FALL_DOWN_M: f32 = 12.0;

// How much of a wall of water is white: long fine strands running down it over white water.
fn falling_foam(world_position: vec3<f32>, normal: vec3<f32>, cycle: f32) -> f32 {
    let across = select(world_position.x, world_position.z, abs(normal.x) > 0.5);
    let uv = vec2<f32>(across / FALL_ACROSS_M, -world_position.y / FALL_DOWN_M);
    let taps = flow_taps(uv, vec2<f32>(0.0, FALL_M_S / FALL_DOWN_M), cycle);
    // Walls are seen near; the mask's second mip keeps the streaks soft without derivatives.
    let mask = dot(
        vec2<f32>(
            textureSampleLevel(water_foam, water_foam_sampler, taps.first, 1.0).r,
            textureSampleLevel(water_foam, water_foam_sampler, taps.second, 1.0).r,
        ),
        taps.weights,
    ) / (taps.weights.x + taps.weights.y);
    let least = bitcast<f32>(constants::CHUNK_FALL_WHITE_LEAST);
    let streaks = vec2<f32>(
        bitcast<f32>(constants::CHUNK_FALL_STREAKS_FROM),
        bitcast<f32>(constants::CHUNK_FALL_STREAKS_TO),
    );
    return mix(least, 1.0, smoothstep(streaks.x, streaks.y, mask));
}

// The body of water under a fragment, premultiplied: what it sends toward the eye as albedo, and
// how much of what lies behind it it hides.
struct WaterBody {
    albedo: vec3<f32>,
    alpha: f32,
}

// Running water carries silt: none below the first speed, a river's full load from the second.
// A lake's drift stays under the first, and a pool of the slowest brook is just past it.
const SILT_FROM_M_S: f32 = 0.15;
const SILT_AT_M_S: f32 = 0.6;
// A river's silt, in metres a ray crosses per e-fold lost: a few metres of it hide the bed, as
// in a lowland river, while a lake and the sea stay as clear as their own water.
const SILT_PER_M: f32 = 0.45;
// What the silt scatters back, linear: a dull, warm grey-green.
const SILT_COLOUR: vec3<f32> = vec3<f32>(0.09, 0.09, 0.055);

// Each channel keeps exp(-extinction x path) of the floor, the path being the depth over the
// cosine of the view, the extinction the water's own absorption plus its silt; what is lost is
// replaced by each one's tint by its share of the extinction. The floor is blended by one alpha,
// so alpha follows the mean of the three.
fn water_body(slot: u32, depth_m: f32, cos_view: f32, foam: f32, silt: f32) -> WaterBody {
    let absorption = water_table[slot * 3u].xyz;
    let scatter = water_table[slot * 3u + 1u].xyz;
    let path = depth_m / max(cos_view, 0.05);
    let clouded = vec3<f32>(SILT_PER_M * silt);
    let extinction = absorption + clouded;
    let lost = vec3<f32>(1.0) - exp(-extinction * path);
    let tint = (scatter * absorption + SILT_COLOUR * clouded) / max(extinction, vec3<f32>(1e-4));
    let body = WaterBody(tint * lost, (lost.x + lost.y + lost.z) / 3.0);
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

const GRAVITY_M_S2: f32 = 9.81;
const TAU: f32 = 6.28318531;

// A beach's slope, one in twenty, which the shoaling phase below is worked out for.
const BEACH_SLOPE: f32 = 0.05;

// The most the shoaling phase may add to a wave's wavenumber, in deep-water wavenumbers.
const SHOALING_PULL: f32 = 1.5;

// Shallower than this the sea breaks: a band of foam travelling in on the swell's crests.
const BREAKER_DEPTH_M: f32 = 1.5;

// The sea's waves at a point: the slope they add, the slope variance a pixel's footprint averaged
// away, and how much of the surface is breaking.
struct SeaWaves {
    slope: vec2<f32>,
    lost: f32,
    breaking: f32,
}

// `water::SEA_WAVES[i]`: cycles per period, radians off the Wind, steepness.
fn sea_wave(i: u32) -> vec3<f32> {
    switch i {
        case 0u: { return vec3<f32>(f32(constants::CHUNK_WAVE0_CYCLES), bitcast<f32>(constants::CHUNK_WAVE0_OFF_WIND), bitcast<f32>(constants::CHUNK_WAVE0_STEEPNESS)); }
        case 1u: { return vec3<f32>(f32(constants::CHUNK_WAVE1_CYCLES), bitcast<f32>(constants::CHUNK_WAVE1_OFF_WIND), bitcast<f32>(constants::CHUNK_WAVE1_STEEPNESS)); }
        case 2u: { return vec3<f32>(f32(constants::CHUNK_WAVE2_CYCLES), bitcast<f32>(constants::CHUNK_WAVE2_OFF_WIND), bitcast<f32>(constants::CHUNK_WAVE2_STEEPNESS)); }
        case 3u: { return vec3<f32>(f32(constants::CHUNK_WAVE3_CYCLES), bitcast<f32>(constants::CHUNK_WAVE3_OFF_WIND), bitcast<f32>(constants::CHUNK_WAVE3_STEEPNESS)); }
        case 4u: { return vec3<f32>(f32(constants::CHUNK_WAVE4_CYCLES), bitcast<f32>(constants::CHUNK_WAVE4_OFF_WIND), bitcast<f32>(constants::CHUNK_WAVE4_STEEPNESS)); }
        default: { return vec3<f32>(f32(constants::CHUNK_WAVE5_CYCLES), bitcast<f32>(constants::CHUNK_WAVE5_OFF_WIND), bitcast<f32>(constants::CHUNK_WAVE5_STEEPNESS)); }
    }
}

// Over depth D a wave of deep-water wavenumber k0 has k = k0 / sqrt(tanh(k0 D)) (Eckart's
// solution of w^2 = g k tanh(k D)); its phase gains the integral of k - k0 as it runs into the
// shallows. Up a beach of `BEACH_SLOPE` that integral is a function of the depth alone,
// 1.04 exp(-2.7 (k0 D)^0.7) / BEACH_SLOPE, fitted to within 0.01: a phase that never tears where
// the depth changes, whose crests shorten and turn along the depth's contours toward the shore.
fn shoaling(u: f32) -> f32 {
    return 1.04 * exp(-2.7 * pow(u, 0.7)) / BEACH_SLOPE;
}

fn sea_waves(xz: vec2<f32>, gradient: WaterGradient, shape: WaterShape, wind: vec4<f32>) -> SeaWaves {
    var out = SeaWaves(vec2<f32>(0.0), 0.0, 0.0);
    let strength = wind.z * shape.sea;
    if strength <= 0.0 {
        return out;
    }
    let downwind = atan2(wind.y, wind.x);
    let footprint = max(length(gradient.ddx), length(gradient.ddy));
    let depth = max(shape.depth_m, 0.05);
    var steepest = 0.0;
    var swell = 0.0;
    var lift = 0.0;
    for (var i = 0u; i < 6u; i++) {
        let wave = sea_wave(i);
        let omega = TAU * wave.x / f32(constants::CHUNK_CLOCK_PERIOD_S);
        let k0 = omega * omega / GRAVITY_M_S2;
        let u = k0 * depth;
        let shallow = 1.0 / sqrt(tanh(u));
        let toward = vec2<f32>(cos(downwind + wave.y), sin(downwind + wave.y));
        // The phase's gradient: the deep-water wavenumber along the wave, and the shoaling
        // phase's along the depth's slope.
        // The filtered depth's slope jumps at every column's centre, so only its direction is
        // taken, the beach assumed `BEACH_SLOPE` steep as the phase is: its size would draw a band
        // across the sea along each row of columns.
        let shoreward = -shape.slope / max(length(shape.slope), 1e-4);
        let across = k0 * (toward + min(shallow - 1.0, SHOALING_PULL) * shoreward);
        let k = length(across);
        // A whole number of cycles per period, so this is continuous where the clock wraps.
        let travel = TAU * fract(clock() * (wave.x / f32(constants::CHUNK_CLOCK_PERIOD_S)));
        let theta = k0 * dot(toward, xz) + shoaling(u) - travel + f32(i) * 1.7;
        // Waves steepen as they shorten, and too short a wave for the pixel is dropped.
        let steepness = wave.z * strength * min(shallow, 2.0);
        let seen = 1.0 - smoothstep(0.25, 0.5, footprint * k / TAU);
        out.slope += steepness * seen * (across / k) * cos(theta);
        out.lost += 0.5 * steepness * steepness * (1.0 - seen);
        lift += steepness * seen * sin(theta);
        steepest += steepness;
        if i == 0u {
            swell = sin(theta);
        }
    }
    // Their steepnesses sum to at most 1 in deep water; where the shallows push past it, all are
    // scaled back so no crest loops.
    let scale = 1.0 / max(steepest, 1.0);
    out.slope = out.slope * scale / (1.0 - lift * scale);
    out.lost *= scale * scale;
    let band = 1.0 - smoothstep(0.2, BREAKER_DEPTH_M, shape.depth_m);
    out.breaking = band * strength * smoothstep(0.4, 0.95, swell);
    return out;
}

struct WaterSurface {
    normal: vec3<f32>,
    perceptual_roughness: f32,
    // The variance of the slopes too fine for the pixel, which `normal` leaves out.
    lost_slope: f32,
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
    sea: SeaWaves,
) -> WaterSurface {
    let base = roughness_of(slot);
    if abs(normal.y) < 0.5 {
        return WaterSurface(normal, base, 0.0);
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
    // Ripples rise with the Wind: half as tall in a breeze, a third taller in the strongest.
    let windy = mix(0.5, 1.35, water_view.wind.z);
    let slope = (a.slope + turned_back) * calm * windy * (1.0 + FLOW_STRENGTH_PER_M_S * length(flow))
        + sea.slope;
    let up = sign(normal.y);
    let rippled = normalize(vec3<f32>(-slope.x, 1.0, -slope.y) * vec3<f32>(1.0, up, 1.0));
    // Beckmann's alpha squared is twice the slope variance; bevy_pbr's alpha is roughness squared.
    let alpha = sqrt(base * base * base * base + 2.0 * (a.lost + b.lost + sea.lost));
    let lost = a.lost + b.lost + sea.lost;
    return WaterSurface(rippled, min(sqrt(alpha), max(base, FAR_ROUGHNESS)), lost);
}

fn fresnel(cos_view: f32) -> f32 {
    return WATER_F0 + (1.0 - WATER_F0) * pow(1.0 - clamp(cos_view, 0.0, 1.0), 5.0);
}

// The normal the view sees on average over a pixel whose slopes are too fine to draw: a viewer
// low over the water sees mostly the facets tilted toward it, which reflect sky higher up and
// reflect less of it. Without it the far water mirrors the pale horizon at nearly full Fresnel
// and reads as a white sheet past level 0.
fn seen_normal(normal: vec3<f32>, view_toward: vec3<f32>, lost_slope: f32) -> vec3<f32> {
    let across = vec2<f32>(view_toward.x, view_toward.z);
    if lost_slope <= 0.0 || dot(across, across) < 1e-6 {
        return normal;
    }
    let toward = normalize(across) * sqrt(lost_slope) * sign(normal.y);
    return normalize(normal + vec3<f32>(toward.x, 0.0, toward.y));
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
