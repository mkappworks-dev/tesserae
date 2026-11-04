//! A volcanic highland. Basalt chimneys are built on start and erupt on a timer; the lava, the
//! water it meets and the wood it burns behave by the rules in the Mods this Game depends on
//! (strata, waters, flora, hearth), so none are declared here.

wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use std::cell::RefCell;
use std::collections::VecDeque;

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::commands::{Fill, SourceFill};
use tesserae::mods::content::{GroundParams, MovementPolicy, WaterParams};
use tesserae::mods::types::{BoxM, Vec3, Voxel};

/// Ground spans `SEA_LEVEL_M` to `SEA_LEVEL_M + RELIEF_M`; a chimney cut from below that band to
/// above it stands on any seed, because a Mod cannot read the terrain it will build on.
const SEA_LEVEL_M: f32 = 24.0;
const RELIEF_M: f32 = 32.0;
const LAKE_M: f32 = 30.0;
const CHIMNEY_FLOOR_M: f32 = SEA_LEVEL_M - 2.0;
const CHIMNEY_TOP_M: f32 = SEA_LEVEL_M + RELIEF_M + 3.0;

/// Chimney positions (lower corner of the 3 by 3 shaft), clear of the origin where the Player
/// spawns. The last one has water beside its lava.
const VENTS_AT_M: [(f32, f32); 3] = [(18.0, -14.0), (-22.0, 10.0), (6.0, 30.0)];
const WET_VENT: usize = 2;

/// Seconds after the build is applied that the first vent erupts, and the stagger between vents.
const FIRST_ERUPTION_S: f32 = 5.0;
const STAGGER_S: f32 = 10.0;
/// A vent erupts for `ERUPTION_S` seconds out of every `PERIOD_S`.
const PERIOD_S: f32 = 30.0;
const ERUPTION_S: f32 = 8.0;

/// The host's caps on one `fill` and on one batch.
const MAX_FILL_VOXELS: u64 = 32_768;
const MAX_BATCH_VOXELS: u64 = 65_536;

#[derive(Default)]
struct State {
    size_m: f32,
    ids: Ids,
    queued: VecDeque<Command>,
    /// Voxels the build covers, and how many have been reported applied or rejected, so the
    /// timer starts on a finished build.
    issued: u64,
    settled: u64,
    built: bool,
    elapsed_s: f32,
    erupting: [bool; VENTS_AT_M.len()],
}

#[derive(Default, Clone, Copy)]
struct Ids {
    basalt: u16,
    wood: u16,
    lava: u16,
    water: u16,
    cube: u8,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Ember;

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

/// The cell on top of a vent's cap where its lava source goes: the cap's middle, or for the wet
/// vent its corner, with the water at the opposite corner.
fn lava_cell(state: &State, vent: usize) -> BoxM {
    let (x, z) = VENTS_AT_M[vent];
    let (dx, dz) = if vent == WET_VENT {
        (0.0, 0.0)
    } else {
        (1.0, 1.0)
    };
    cell(state, (x + dx, CHIMNEY_TOP_M, z + dz))
}

fn cell(state: &State, at: (f32, f32, f32)) -> BoxM {
    let s = state.size_m;
    region(at, (at.0 + s, at.1 + s, at.2 + s))
}

/// Whether `vent` is erupting `elapsed_s` seconds after the build was applied.
fn is_erupting(vent: usize, elapsed_s: f32) -> bool {
    let since = elapsed_s - FIRST_ERUPTION_S - STAGGER_S * vent as f32;
    since >= 0.0 && since % PERIOD_S < ERUPTION_S
}

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

/// Three chimneys, a wooden deck round the first that its lava will reach, and the wet vent's
/// water. The Server applies Commands in order, a few Voxels a tick, so the chimney nearest the
/// spawn goes up first; each is one piece, since the deck replaces part of its shaft.
fn build(state: &State) -> Vec<Command> {
    let (s, ids) = (state.size_m, state.ids);
    let solid = |material| Voxel {
        material,
        shape: ids.cube,
        orientation: 0,
    };
    let shaft_split = CHIMNEY_TOP_M - 6.0;
    let pieces = VENTS_AT_M
        .iter()
        .enumerate()
        .map(|(vent, &(x, z))| {
            let mut piece = fill(
                region(
                    (x - 1.0, CHIMNEY_FLOOR_M, z - 1.0),
                    (x + 4.0, shaft_split, z + 4.0),
                ),
                s,
                solid(ids.basalt),
            );
            piece.extend(fill(
                region((x, shaft_split, z), (x + 3.0, CHIMNEY_TOP_M, z + 3.0)),
                s,
                solid(ids.basalt),
            ));
            if vent == 0 {
                // Wood where the first vent's lava comes down, which heat from it will light.
                piece.extend(fill(
                    region(
                        (x - 2.0, CHIMNEY_TOP_M - 4.0, z - 2.0),
                        (x + 5.0, CHIMNEY_TOP_M - 3.0, z + 5.0),
                    ),
                    s,
                    solid(ids.wood),
                ));
            }
            if vent == WET_VENT {
                piece.push(Command::PlaceSources(SourceFill {
                    region: cell(state, (x + 2.0, CHIMNEY_TOP_M, z + 2.0)),
                    material: ids.water,
                }));
            }
            piece
        })
        .collect();
    nearest_first(pieces)
}

impl Guest for Ember {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.movement = Some(MovementPolicy {
            allow_flight: true,
            allow_physics: true,
            start_in_flight: false,
        });
        world.sea_level_m = SEA_LEVEL_M;
        world.relief_m = RELIEF_M;
        world.water = Some(WaterParams {
            level_m: LAKE_M,
            material: "waters:water".into(),
        });
        world.ground = Some(GroundParams {
            rock: "strata:basalt".into(),
            soil: "strata:stone".into(),
            surface: "hearth:ash".into(),
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
        }
    }

    fn start(world: WorldInfo, ids: Registered) {
        let material =
            |name: &str| ids.materials.iter().find(|r| r.name == name).unwrap().id as u16;
        let shape = |name: &str| ids.shapes.iter().find(|r| r.name == name).unwrap().id as u8;
        let mut state = State {
            size_m: world.voxel_size_m,
            ids: Ids {
                basalt: material("strata:basalt"),
                wood: material("flora:wood"),
                lava: material("hearth:lava"),
                water: material("waters:water"),
                cube: shape("core:cube"),
            },
            ..State::default()
        };
        // A reopened World already holds its chimneys; the timer simply starts again.
        if world.reopened {
            state.built = true;
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
            let mut batch = Vec::new();
            for vent in 0..VENTS_AT_M.len() {
                let now = is_erupting(vent, state.elapsed_s);
                if now == state.erupting[vent] {
                    continue;
                }
                state.erupting[vent] = now;
                let at = lava_cell(state, vent);
                batch.push(if now {
                    Command::PlaceSources(SourceFill {
                        region: at,
                        material: state.ids.lava,
                    })
                } else {
                    Command::RemoveSources(at)
                });
            }
            batch
        })
    }
}

export!(Ember);
