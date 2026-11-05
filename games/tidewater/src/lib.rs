//! Islands and shallow seas whose sea level rises and falls slowly. The tide is a layer of water
//! sources added or taken away above the sea, a step at a time; the water itself, the ground and
//! the kelp and the ore come from the Mods this Game depends on (strata, waters, flora, crystals).

wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use std::cell::RefCell;
use std::collections::VecDeque;

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::commands::{Fill, SourceFill};
use tesserae::mods::content::{
    BiomeDef, CaveParams, ClimateParams, GroundParams, MovementPolicy, OreDef, SkyFog,
    SmoothingParams, WaterParams,
};
use tesserae::mods::types::{BoxM, Vec3, Voxel};

/// Ground spans `SEA_LEVEL_M` to `SEA_LEVEL_M + RELIEF_M`; the sea stands at `SEA_M` inside that
/// band, so ground above it is an island and ground below it a sea floor.
const SEA_LEVEL_M: f32 = 16.0;
const RELIEF_M: f32 = 24.0;
const SEA_M: f32 = 26.0;

/// High tide stands this far above `SEA_M`. The Player's spawn eye is `SPAWN_CLEARANCE_M` above
/// the sea at the lowest (the engine's 2 m), so the tide cannot reach it.
const TIDE_RANGE_M: f32 = 1.0;
const SPAWN_CLEARANCE_M: f32 = 2.0;
const _: () = assert!(TIDE_RANGE_M < SPAWN_CLEARANCE_M);
/// The tide acts on this square, centred on the origin.
const TIDE_HALF_M: f32 = 24.0;
/// A tide step: one layer of sources is added or removed this often.
const STEP_S: f32 = 8.0;

/// Kelp stalks (lower corner), clear of the origin, rooted below the sea floor band.
const KELP_AT_M: [(f32, f32); 6] = [
    (10.0, 6.0),
    (-14.0, 9.0),
    (7.0, -16.0),
    (-9.0, -12.0),
    (20.0, -4.0),
    (-20.0, -6.0),
];
const KELP_FLOOR_M: f32 = SEA_LEVEL_M - 2.0;
const KELP_TOP_M: f32 = SEA_M - 1.0;

/// The host's caps on one `fill` and on one batch.
const MAX_FILL_VOXELS: u64 = 32_768;
const MAX_BATCH_VOXELS: u64 = 65_536;

#[derive(Default)]
struct State {
    size_m: f32,
    ids: Ids,
    queued: VecDeque<Command>,
    /// How many layers of the tide band the voxel size allows, and how many are up.
    layers: u32,
    up: u32,
    rising: bool,
    next_step_s: f32,
    elapsed_s: f32,
}

#[derive(Default, Clone, Copy)]
struct Ids {
    kelp: u16,
    water: u16,
    cube: u8,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Tidewater;

fn point(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

fn region(min: (f32, f32, f32), max: (f32, f32, f32)) -> BoxM {
    BoxM {
        min: point(min.0, min.1, min.2),
        max: point(max.0, max.1, max.2),
    }
}

/// The Voxels a region covers at `size` m: those whose lower corner is inside.
fn voxels_in(region: &BoxM, size: f32) -> u64 {
    let n = |a: f32, b: f32| ((b / size).ceil() - (a / size).ceil()).max(0.0) as u64;
    n(region.min.x, region.max.x) * n(region.min.y, region.max.y) * n(region.min.z, region.max.z)
}

fn voxels(command: &Command, size: f32) -> u64 {
    match command {
        Command::Fill(f) => voxels_in(&f.region, size),
        Command::Place(p) => p.len() as u64,
        Command::PlaceSources(f) => voxels_in(&f.region, size),
        Command::RemoveSources(r) | Command::Ignite(r) | Command::Extinguish(r) => {
            voxels_in(r, size)
        }
    }
}

/// `fill` over `region` cut along x into pieces within the fill cap.
fn fill(region: BoxM, size: f32, voxel: Voxel) -> Vec<Command> {
    let across = |a: f32, b: f32| ((b / size).ceil() - (a / size).ceil()).max(1.0);
    let column = across(region.min.y, region.max.y) * across(region.min.z, region.max.z);
    let step = (MAX_FILL_VOXELS as f32 / column).floor().max(1.0) * size;
    let mut pieces = Vec::new();
    let mut x = region.min.x;
    while x < region.max.x {
        let end = (x + step).min(region.max.x);
        pieces.push(Command::Fill(Fill {
            region: BoxM {
                min: point(x, region.min.y, region.min.z),
                max: point(end, region.max.y, region.max.z),
            },
            voxel,
        }));
        x = end;
    }
    pieces
}

/// Queued Commands up to the batch cap, in order.
fn next_batch(state: &mut State) -> Vec<Command> {
    let mut batch = Vec::new();
    let mut total = 0;
    while let Some(next) = state.queued.front() {
        let n = voxels(next, state.size_m);
        if !batch.is_empty() && total + n > MAX_BATCH_VOXELS {
            break;
        }
        total += n;
        batch.extend(state.queued.pop_front());
    }
    batch
}

/// The tide's layer `index`, one Voxel thick, as strips within the fill cap.
fn layer(state: &State, index: u32, make: impl Fn(BoxM) -> Command) -> Vec<Command> {
    let s = state.size_m;
    let low = SEA_M + index as f32 * s;
    let across = ((2.0 * TIDE_HALF_M / s).ceil()).max(1.0);
    let step = (MAX_FILL_VOXELS as f32 / across).floor().max(1.0) * s;
    let mut pieces = Vec::new();
    let mut x = -TIDE_HALF_M;
    while x < TIDE_HALF_M {
        let end = (x + step).min(TIDE_HALF_M);
        pieces.push(make(region(
            (x, low, -TIDE_HALF_M),
            (end, low + s, TIDE_HALF_M),
        )));
        x = end;
    }
    pieces
}

fn raise(state: &State, index: u32) -> Vec<Command> {
    let material = state.ids.water;
    layer(state, index, |region| {
        Command::PlaceSources(SourceFill { region, material })
    })
}

fn lower(state: &State, index: u32) -> Vec<Command> {
    layer(state, index, Command::RemoveSources)
}

/// The next tide step: add a layer on the way up, take one away on the way down, and turn
/// round at either end.
fn step(state: &mut State) -> Vec<Command> {
    if state.rising {
        if state.up < state.layers {
            state.up += 1;
            return raise(state, state.up - 1);
        }
        state.rising = false;
    } else if state.up > 0 {
        state.up -= 1;
        return lower(state, state.up);
    } else {
        state.rising = true;
    }
    Vec::new()
}

/// Kelp rooted below the sea floor and reaching for the surface; where the ground is higher
/// the stalk is buried and invisible.
fn plant(state: &State) -> Vec<Command> {
    let kelp = Voxel {
        material: state.ids.kelp,
        shape: state.ids.cube,
        orientation: 0,
    };
    KELP_AT_M
        .iter()
        .flat_map(|&(x, z)| {
            fill(
                region((x, KELP_FLOOR_M, z), (x + 1.0, KELP_TOP_M, z + 1.0)),
                state.size_m,
                kelp,
            )
        })
        .collect()
}

impl Guest for Tidewater {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.movement = Some(MovementPolicy {
            allow_flight: true,
            allow_physics: true,
            start_in_flight: false,
        });
        world.sea_level_m = SEA_LEVEL_M;
        world.relief_m = RELIEF_M;
        world.feature_size_m = 64.0;
        world.caves = Some(CaveParams {
            feature_size_m: 48.0,
            radius_m: 2.5,
            floor_m: SEA_LEVEL_M,
            octaves: 2,
        });
        world.smoothing = Some(SmoothingParams { min_rise_m: 0.5 });
        world.water = Some(WaterParams {
            level_m: SEA_M,
            material: "waters:water".into(),
        });
        world.ground = Some(GroundParams {
            rock: "strata:stone".into(),
            soil: "strata:soil".into(),
            surface: "strata:turf".into(),
        });
        // Smaller than the engine's 512 m, so one island hop meets a border.
        world.climate = Some(ClimateParams {
            temperature_size_m: 256.0,
            humidity_size_m: 256.0,
        });
        Declaration {
            materials: Vec::new(),
            shapes: Vec::new(),
            appearance: Vec::new(),
            world: Some(world),
            view: None,
            fluids: Vec::new(),
            reactions: Vec::new(),
            combustion: Vec::new(),
            tools: Vec::new(),
            items: Vec::new(),
            biomes: vec![
                BiomeDef {
                    name: "lagoon".into(),
                    temperature: 0.5,
                    humidity: 0.5,
                    ground: None,
                    sky_fog: Some(SkyFog {
                        sky: (118, 150, 170),
                        fog: (132, 172, 192),
                    }),
                    cave_scale: Some(1.4),
                    surface_features: Vec::new(),
                },
                BiomeDef {
                    name: "salt_flats".into(),
                    temperature: 0.7,
                    humidity: 0.3,
                    ground: Some(GroundParams {
                        rock: "strata:stone".into(),
                        soil: "strata:clay".into(),
                        surface: "strata:sand".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (170, 164, 150),
                        fog: (190, 184, 166),
                    }),
                    cave_scale: Some(0.0),
                    surface_features: Vec::new(),
                },
                BiomeDef {
                    name: "cold_shallows".into(),
                    temperature: 0.3,
                    humidity: 0.55,
                    ground: Some(GroundParams {
                        rock: "strata:basalt".into(),
                        soil: "strata:stone".into(),
                        surface: "strata:stone".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (144, 160, 180),
                        fog: (168, 186, 206),
                    }),
                    cave_scale: None,
                    surface_features: Vec::new(),
                },
            ],
            survival: None,
            ores: vec![OreDef {
                material: "crystals:aquamarine".into(),
                min_depth_m: 6.0,
                max_depth_m: 22.0,
                vein_size_m: 5.0,
                rarity: 0.06,
                biomes: Vec::new(),
            }],
        }
    }

    fn start(world: WorldInfo, ids: Registered) {
        let material =
            |name: &str| ids.materials.iter().find(|r| r.name == name).unwrap().id as u16;
        let shape = |name: &str| ids.shapes.iter().find(|r| r.name == name).unwrap().id as u8;
        let mut state = State {
            size_m: world.voxel_size_m,
            ids: Ids {
                kelp: material("flora:kelp"),
                water: material("waters:water"),
                cube: shape("core:cube"),
            },
            rising: true,
            next_step_s: STEP_S,
            ..State::default()
        };
        // Whole Voxel layers that fit in the range; a Voxel taller than it leaves no tide.
        state.layers = (TIDE_RANGE_M / state.size_m).floor() as u32;
        if world.reopened {
            // The saved sea may be mid-tide: take every layer away so the tide starts low.
            for index in 0..state.layers {
                state.queued.extend(lower(&state, index));
            }
        } else {
            state.queued = plant(&state).into();
        }
        STATE.replace(state);
    }

    fn tick(ctx: TickContext) -> Vec<Command> {
        STATE.with_borrow_mut(|state| {
            state.elapsed_s += ctx.dt_s;
            if state.elapsed_s >= state.next_step_s {
                state.next_step_s += STEP_S;
                let commands = step(state);
                state.queued.extend(commands);
            }
            next_batch(state)
        })
    }
}

export!(Tidewater);
