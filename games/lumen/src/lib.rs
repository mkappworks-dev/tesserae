//! An enclosed garden of glass and light. The garden is built on start; its lanterns then light
//! up one after another on a timer. Glass, lanterns and the slab, stair and fence Shapes come from
//! the Mods this Game depends on (strata, glassworks).

wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use std::cell::RefCell;
use std::collections::VecDeque;

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::commands::Fill;
use tesserae::mods::content::{GroundParams, MovementPolicy};
use tesserae::mods::types::{BoxM, Vec3, Voxel};

/// Ground spans `SEA_LEVEL_M` to `SEA_LEVEL_M + RELIEF_M`; the garden's floor is levelled above
/// that, because a Mod cannot read the terrain it will build on.
const SEA_LEVEL_M: f32 = 24.0;
const RELIEF_M: f32 = 1.0;
const FLOOR_M: f32 = SEA_LEVEL_M + RELIEF_M + 1.0;
const WALL_M: f32 = 5.0;

/// The garden's footprint: 24 m square, clear of the origin where the Player spawns, with a gap in
/// the wall nearest the origin.
const GARDEN_X_M: (f32, f32) = (-12.0, 12.0);
const GARDEN_Z_M: (f32, f32) = (14.0, 38.0);
const DOOR_X_M: (f32, f32) = (-2.0, 2.0);

/// Lanterns stand on posts round a point, in the order they light.
const LANTERNS_AT_M: [(f32, f32); 8] = [
    (8.0, 26.0),
    (6.0, 32.0),
    (0.0, 34.0),
    (-6.0, 32.0),
    (-8.0, 26.0),
    (-6.0, 20.0),
    (0.0, 18.0),
    (6.0, 20.0),
];
const POST_HEIGHT_M: f32 = 3.0;

/// Seconds between one lantern lighting and the next, and how long all stay lit before they go
/// dark together and the sequence starts again.
const LIGHT_STEP_S: f32 = 2.0;
const HOLD_S: f32 = 6.0;

/// The host's caps on one `fill` and on one batch.
const MAX_FILL_VOXELS: u64 = 32_768;
const MAX_BATCH_VOXELS: u64 = 65_536;

#[derive(Default)]
struct State {
    size_m: f32,
    ids: Ids,
    queued: VecDeque<Command>,
    issued: u64,
    settled: u64,
    built: bool,
    elapsed_s: f32,
    next_s: f32,
    lit: usize,
}

#[derive(Default, Clone, Copy)]
struct Ids {
    stone: u16,
    soil: u16,
    turf: u16,
    glass: u16,
    frosted: u16,
    lantern: u16,
    cube: u8,
    slab: u8,
    stair: u8,
    fence: u8,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Lumen;

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

/// The cell a lantern sits in, on top of its post.
fn lantern_at(index: usize) -> BoxM {
    let (x, z) = LANTERNS_AT_M[index];
    let y = FLOOR_M + POST_HEIGHT_M;
    region((x, y, z), (x + 1.0, y + 1.0, z + 1.0))
}

/// The Commands for the next point on the light timer: light a lantern, or once all are lit
/// and have been held, put them all out.
fn light(state: &mut State) -> Vec<Command> {
    let s = state.size_m;
    let cube = state.ids.cube;
    if state.lit < LANTERNS_AT_M.len() {
        let region = lantern_at(state.lit);
        state.lit += 1;
        state.next_s += if state.lit == LANTERNS_AT_M.len() {
            HOLD_S
        } else {
            LIGHT_STEP_S
        };
        return fill(
            region,
            s,
            Voxel {
                material: state.ids.lantern,
                shape: cube,
                orientation: 0,
            },
        );
    }
    state.lit = 0;
    state.next_s += LIGHT_STEP_S;
    (0..LANTERNS_AT_M.len())
        .flat_map(|index| {
            fill(
                lantern_at(index),
                s,
                Voxel {
                    material: 0,
                    shape: cube,
                    orientation: 0,
                },
            )
        })
        .collect()
}

/// The levelled floor, glass walls with a door, a frosted roof, a slab path and stair, and the
/// unlit lantern posts.
fn build(state: &State) -> Vec<Command> {
    let (s, ids) = (state.size_m, state.ids);
    let voxel = |material, shape, orientation| Voxel {
        material,
        shape,
        orientation,
    };
    let cube = |material| voxel(material, ids.cube, 0);
    let (x, z) = (GARDEN_X_M, GARDEN_Z_M);
    let top = FLOOR_M + WALL_M;
    let mut commands = Vec::new();
    // Ground from the bottom of the ground band to the floor, with turf on top. Nothing needs
    // clearing above: the floor is above the highest ground, so what is there is air.
    commands.extend(fill(
        region(
            (x.0 - 1.0, SEA_LEVEL_M, z.0 - 1.0),
            (x.1 + 1.0, FLOOR_M - 1.0, z.1 + 1.0),
        ),
        s,
        cube(ids.soil),
    ));
    commands.extend(fill(
        region(
            (x.0 - 1.0, FLOOR_M - 1.0, z.0 - 1.0),
            (x.1 + 1.0, FLOOR_M, z.1 + 1.0),
        ),
        s,
        cube(ids.turf),
    ));
    // Walls, all four sides, then the doorway cut back out of the near one.
    for wall in [
        ((x.0, z.0), (x.1, z.0 + 1.0)),
        ((x.0, z.1 - 1.0), (x.1, z.1)),
        ((x.0, z.0), (x.0 + 1.0, z.1)),
        ((x.1 - 1.0, z.0), (x.1, z.1)),
    ] {
        commands.extend(fill(
            region((wall.0.0, FLOOR_M, wall.0.1), (wall.1.0, top, wall.1.1)),
            s,
            cube(ids.glass),
        ));
    }
    commands.extend(fill(
        region((DOOR_X_M.0, FLOOR_M, z.0), (DOOR_X_M.1, top, z.0 + 1.0)),
        s,
        cube(0),
    ));
    commands.extend(fill(
        region((x.0, top, z.0), (x.1, top + 1.0, z.1)),
        s,
        cube(ids.frosted),
    ));
    // A path of slabs from the door to the middle, with a stair up to it.
    commands.extend(fill(
        region((-1.0, FLOOR_M, z.0 + 1.0), (1.0, FLOOR_M + s, z.0 + 12.0)),
        s,
        voxel(ids.stone, ids.slab, 0),
    ));
    commands.extend(fill(
        region((-1.0, FLOOR_M, z.0), (1.0, FLOOR_M + s, z.0 + 1.0)),
        s,
        voxel(ids.stone, ids.stair, 0),
    ));
    // Lantern posts: stone in the fence Shape, with nothing yet where the lantern will go.
    for &(px, pz) in &LANTERNS_AT_M {
        commands.extend(fill(
            region(
                (px, FLOOR_M, pz),
                (px + 1.0, FLOOR_M + POST_HEIGHT_M, pz + 1.0),
            ),
            s,
            voxel(ids.stone, ids.fence, 0),
        ));
    }
    commands
}

impl Guest for Lumen {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.movement = Some(MovementPolicy {
            allow_flight: true,
            allow_physics: true,
            start_in_flight: false,
        });
        world.sea_level_m = SEA_LEVEL_M;
        world.relief_m = RELIEF_M;
        world.caves = None;
        world.ground = Some(GroundParams {
            rock: "strata:stone".into(),
            soil: "strata:soil".into(),
            surface: "strata:turf".into(),
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
        }
    }

    fn start(world: WorldInfo, ids: Registered) {
        let material =
            |name: &str| ids.materials.iter().find(|r| r.name == name).unwrap().id as u16;
        let shape = |name: &str| ids.shapes.iter().find(|r| r.name == name).unwrap().id as u8;
        let mut state = State {
            size_m: world.voxel_size_m,
            ids: Ids {
                stone: material("strata:stone"),
                soil: material("strata:soil"),
                turf: material("strata:turf"),
                glass: material("glassworks:glass"),
                frosted: material("glassworks:frosted_glass"),
                lantern: material("glassworks:lantern"),
                cube: shape("core:cube"),
                slab: shape("glassworks:slab"),
                stair: shape("glassworks:stair"),
                fence: shape("glassworks:fence"),
            },
            next_s: LIGHT_STEP_S,
            ..State::default()
        };
        // A reopened World already holds the garden, with whatever lanterns were lit; the
        // sequence restarts from dark by clearing them first.
        if world.reopened {
            state.built = true;
            state.lit = LANTERNS_AT_M.len();
            state.next_s = 0.0;
        } else {
            let commands = build(&state);
            state.issued = commands.iter().map(|c| voxels(c, state.size_m)).sum();
            state.queued = commands.into();
        }
        STATE.replace(state);
    }

    fn tick(ctx: TickContext) -> Vec<Command> {
        STATE.with_borrow_mut(|state| {
            state.settled += u64::from(ctx.last.applied) + u64::from(ctx.last.rejected);
            if !state.built {
                if state.queued.is_empty() && state.settled >= state.issued {
                    state.built = true;
                } else {
                    return next_batch(state);
                }
            }
            state.elapsed_s += ctx.dt_s;
            if state.elapsed_s >= state.next_s {
                return light(state);
            }
            Vec::new()
        })
    }
}

export!(Lumen);
