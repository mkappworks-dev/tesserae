//! Gentle land and a glade of trees. Every Material and Shape comes from the Mods it depends on
//! (strata, flora, glassworks); this Game declares only its World and one behaviour.

wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use std::cell::RefCell;
use std::collections::VecDeque;

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::commands::Fill;
use tesserae::mods::content::{
    BiomeDef, ClimateParams, GroundParams, MovementPolicy, OreDef, SkyFog,
};
use tesserae::mods::types::{BoxM, Vec3, Voxel};

/// Ground spans `SEA_LEVEL_M` to `SEA_LEVEL_M + RELIEF_M`, so a trunk or pad cut at fixed heights
/// lands in or above the ground at any seed: a Mod cannot read the terrain it will build on.
const SEA_LEVEL_M: f32 = 24.0;
const RELIEF_M: f32 = 10.0;

/// Wide enough that the glade seldom stands across a Biome border.
const CLIMATE_SIZE_M: f32 = 400.0;

/// The glade: ground levelled to `GLADE_TOP_M`, clear of the origin where the Player spawns.
const GLADE_X_M: (f32, f32) = (8.0, 24.0);
const GLADE_Z_M: (f32, f32) = (8.0, 24.0);
const GLADE_FLOOR_M: f32 = SEA_LEVEL_M - 2.0;
const GLADE_TOP_M: f32 = SEA_LEVEL_M + RELIEF_M / 2.0;
/// Above the ground band there is only air, so clearing stops at its top.
const GLADE_CLEAR_M: f32 = SEA_LEVEL_M + RELIEF_M;

const TREES_AT_M: [(f32, f32); 3] = [(11.0, 11.0), (12.0, 20.0), (21.0, 12.0)];
const TRUNK_HEIGHT_M: f32 = 5.0;
/// Where the gazebo's four posts stand, and how far apart (a whole number of metres).
const GAZEBO_AT_M: (f32, f32) = (17.0, 17.0);
const GAZEBO_SPAN_M: f32 = 4.0;
const POST_HEIGHT_M: f32 = 3.0;

/// The host's caps on one `fill` and on one batch.
const MAX_FILL_VOXELS: u64 = 32_768;
const MAX_BATCH_VOXELS: u64 = 65_536;

/// Climate clusters near the middle of its square, so the points stay between 0.2 and 0.8 or a
/// Biome would seldom be the nearest. The World has no caves, so no Biome sets a `cave_scale`.
fn biomes() -> Vec<BiomeDef> {
    let ground = |rock: &str, soil: &str, surface: &str| {
        Some(GroundParams {
            rock: rock.into(),
            soil: soil.into(),
            surface: surface.into(),
        })
    };
    vec![
        BiomeDef {
            name: "meadow".into(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: Some(SkyFog {
                sky: (128, 152, 172),
                fog: (142, 168, 192),
            }),
            cave_scale: None,
        },
        BiomeDef {
            name: "heath".into(),
            temperature: 0.3,
            humidity: 0.7,
            ground: ground("strata:stone", "strata:clay", "strata:soil"),
            sky_fog: Some(SkyFog {
                sky: (150, 148, 150),
                fog: (170, 168, 170),
            }),
            cave_scale: None,
        },
        BiomeDef {
            name: "dunes".into(),
            temperature: 0.75,
            humidity: 0.25,
            ground: ground("strata:stone", "strata:sand", "strata:sand"),
            sky_fog: Some(SkyFog {
                sky: (174, 160, 134),
                fog: (194, 178, 150),
            }),
            cave_scale: None,
        },
    ]
}

#[derive(Default)]
struct State {
    size_m: f32,
    ids: Ids,
    queued: VecDeque<Command>,
}

#[derive(Default, Clone, Copy)]
struct Ids {
    clay: u16,
    turf: u16,
    wood: u16,
    leaves: u16,
    glass: u16,
    cube: u8,
    fence: u8,
    slab: u8,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Meadow;

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

/// Width of the strips the glade's ground is laid in, so the near edge goes down first.
const STRIP_M: f32 = 4.0;

/// How far the nearest point of `region` is from the spawn at the origin, over the ground.
fn distance_m(region: &BoxM) -> f32 {
    let along = |low: f32, high: f32| {
        if low > 0.0 {
            low
        } else if high < 0.0 {
            -high
        } else {
            0.0
        }
    };
    along(region.min.x, region.max.x).hypot(along(region.min.z, region.max.z))
}

fn nearest(command: &Command) -> f32 {
    match command {
        Command::Fill(f) => distance_m(&f.region),
        Command::PlaceSources(f) => distance_m(&f.region),
        Command::RemoveSources(r) | Command::Ignite(r) | Command::Extinguish(r) => distance_m(r),
        Command::Place(p) => p
            .iter()
            .map(|at| at.at.x.hypot(at.at.z))
            .fold(f32::MAX, f32::min),
    }
}

/// The Server applies a Mod's Commands in order, a few Voxels a tick, so the pieces nearest the
/// Player go first. A piece is Commands that must land in order; pieces are stable-sorted.
fn nearest_first(mut pieces: Vec<Vec<Command>>) -> Vec<Command> {
    let key = |piece: &Vec<Command>| piece.iter().map(nearest).fold(f32::MAX, f32::min);
    pieces.sort_by(|a, b| key(a).total_cmp(&key(b)));
    pieces.into_iter().flatten().collect()
}

/// `region` cut along z into strips of `STRIP_M`.
fn strips(region: BoxM) -> Vec<BoxM> {
    let mut pieces = Vec::new();
    let mut z = region.min.z;
    while z < region.max.z {
        let end = (z + STRIP_M).min(region.max.z);
        pieces.push(BoxM {
            min: point(region.min.x, region.min.y, z),
            max: point(region.max.x, region.max.y, end),
        });
        z = end;
    }
    pieces
}

/// The glade, its trees and the gazebo: the levelled ground first, near strips before far ones,
/// then each tree and the gazebo, nearest first. Structures stand on air the ground pieces clear,
/// so they come after all of them.
fn build(state: &State) -> Vec<Command> {
    let (s, ids) = (state.size_m, state.ids);
    let voxel = |material, shape| Voxel {
        material,
        shape,
        orientation: 0,
    };
    let (x, z) = (GLADE_X_M, GLADE_Z_M);
    let band = |low: f32, high: f32| region((x.0, low, z.0), (x.1, high, z.1));
    let layers = [
        (GLADE_FLOOR_M, GLADE_TOP_M - 1.0, voxel(ids.clay, ids.cube)),
        (GLADE_TOP_M - 1.0, GLADE_TOP_M, voxel(ids.turf, ids.cube)),
        (GLADE_TOP_M, GLADE_CLEAR_M, voxel(0, ids.cube)),
    ];
    let ground: Vec<Vec<Command>> = strips(band(GLADE_FLOOR_M, GLADE_CLEAR_M))
        .into_iter()
        .map(|strip| {
            layers
                .iter()
                .flat_map(|&(low, high, material)| {
                    let mut layer = strip;
                    layer.min.y = low;
                    layer.max.y = high;
                    fill(layer, s, material)
                })
                .collect()
        })
        .collect();

    let mut structures: Vec<Vec<Command>> = Vec::new();
    let crown_y = GLADE_TOP_M + TRUNK_HEIGHT_M;
    for (tx, tz) in TREES_AT_M {
        let mut tree = fill(
            region((tx, GLADE_TOP_M, tz), (tx + 1.0, crown_y, tz + 1.0)),
            s,
            voxel(ids.wood, ids.cube),
        );
        tree.extend(fill(
            region(
                (tx - 1.0, crown_y - 1.0, tz - 1.0),
                (tx + 2.0, crown_y + 1.0, tz + 2.0),
            ),
            s,
            voxel(ids.leaves, ids.cube),
        ));
        tree.extend(fill(
            region((tx, crown_y + 1.0, tz), (tx + 1.0, crown_y + 2.0, tz + 1.0)),
            s,
            voxel(ids.leaves, ids.cube),
        ));
        structures.push(tree);
    }

    // Wooden posts in a glassworks fence Shape, under a glass slab roof: Material and Shape
    // come from different Mods.
    let (gx, gz) = GAZEBO_AT_M;
    let mut gazebo = Vec::new();
    for (px, pz) in [
        (gx, gz),
        (gx + GAZEBO_SPAN_M, gz),
        (gx, gz + GAZEBO_SPAN_M),
        (gx + GAZEBO_SPAN_M, gz + GAZEBO_SPAN_M),
    ] {
        gazebo.extend(fill(
            region(
                (px, GLADE_TOP_M, pz),
                (px + 1.0, GLADE_TOP_M + POST_HEIGHT_M, pz + 1.0),
            ),
            s,
            voxel(ids.wood, ids.fence),
        ));
    }
    gazebo.extend(fill(
        region(
            (gx, GLADE_TOP_M + POST_HEIGHT_M, gz),
            (
                gx + GAZEBO_SPAN_M + 1.0,
                GLADE_TOP_M + POST_HEIGHT_M + 1.0,
                gz + GAZEBO_SPAN_M + 1.0,
            ),
        ),
        s,
        voxel(ids.glass, ids.slab),
    ));
    structures.push(gazebo);

    let mut commands = nearest_first(ground);
    commands.extend(nearest_first(structures));
    commands
}

impl Guest for Meadow {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.movement = Some(MovementPolicy {
            allow_flight: true,
            allow_physics: true,
            start_in_flight: false,
        });
        world.sea_level_m = SEA_LEVEL_M;
        world.relief_m = RELIEF_M;
        world.feature_size_m = 128.0;
        world.caves = None;
        world.ground = Some(GroundParams {
            rock: "strata:stone".into(),
            soil: "strata:clay".into(),
            surface: "strata:turf".into(),
        });
        world.climate = Some(ClimateParams {
            temperature_size_m: CLIMATE_SIZE_M,
            humidity_size_m: CLIMATE_SIZE_M,
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
            biomes: biomes(),
            ores: vec![OreDef {
                material: "strata:basalt".into(),
                min_depth_m: 6.0,
                max_depth_m: 36.0,
                vein_size_m: 5.0,
                rarity: 0.3,
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
                clay: material("strata:clay"),
                turf: material("strata:turf"),
                wood: material("flora:wood"),
                leaves: material("flora:leaves"),
                glass: material("glassworks:glass"),
                cube: shape("core:cube"),
                fence: shape("glassworks:fence"),
                slab: shape("glassworks:slab"),
            },
            queued: VecDeque::new(),
        };
        // A reopened World already holds the glade.
        if !world.reopened {
            state.queued = build(&state).into();
        }
        STATE.replace(state);
    }

    fn tick(_ctx: TickContext) -> Vec<Command> {
        STATE.with_borrow_mut(next_batch)
    }
}

export!(Meadow);
