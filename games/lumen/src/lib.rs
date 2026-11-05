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
use tesserae::mods::content::{
    BiomeDef, ClimateParams, GroundParams, MovementPolicy, OreDef, SkyFog,
};
use tesserae::mods::types::{BoxM, Vec3, Voxel};

/// Ground spans `SEA_LEVEL_M` to `SEA_LEVEL_M + RELIEF_M`; the garden's floor is levelled above
/// that, because a Mod cannot read the terrain it will build on.
const SEA_LEVEL_M: f32 = 24.0;
const RELIEF_M: f32 = 1.0;
const FLOOR_M: f32 = SEA_LEVEL_M + RELIEF_M + 1.0;
const WALL_M: f32 = 5.0;

/// Much wider than the garden, so its walls seldom stand across a Biome border.
const CLIMATE_SIZE_M: f32 = 480.0;

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

/// Climate clusters near the middle of its square: points outside 0.2 to 0.8 seldom win.
/// The World has no caves, so no Biome sets a `cave_scale`.
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
            name: "garden".into(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: Some(SkyFog {
                sky: (130, 150, 160),
                fog: (146, 168, 180),
            }),
            cave_scale: None,
            surface_features: Vec::new(),
        },
        BiomeDef {
            name: "arid_court".into(),
            temperature: 0.78,
            humidity: 0.25,
            ground: ground("strata:stone", "strata:sand", "strata:sand"),
            sky_fog: Some(SkyFog {
                sky: (172, 158, 132),
                fog: (192, 176, 146),
            }),
            cave_scale: None,
            surface_features: Vec::new(),
        },
        BiomeDef {
            name: "frost_court".into(),
            temperature: 0.22,
            humidity: 0.65,
            ground: ground("strata:stone", "strata:soil", "frost:snow"),
            sky_fog: Some(SkyFog {
                sky: (152, 166, 188),
                fog: (178, 192, 212),
            }),
            cave_scale: None,
            surface_features: Vec::new(),
        },
    ]
}

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

/// Width of the strips the ground is laid in, so the near edge goes down first.
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

/// The levelled floor, glass walls with a door, a frosted roof, a slab path and stair, and the
/// unlit lantern posts. The Server applies Commands in order, a few Voxels a tick, so each group
/// is laid nearest the spawn first: the floor, then walls, path and posts, then the roof. A piece
/// is Commands that must land in order: the door is cut after its wall, the stair after the door.
fn build(state: &State) -> Vec<Command> {
    let (s, ids) = (state.size_m, state.ids);
    let voxel = |material, shape| Voxel {
        material,
        shape,
        orientation: 0,
    };
    let cube = |material| voxel(material, ids.cube);
    let (x, z) = (GARDEN_X_M, GARDEN_Z_M);
    let top = FLOOR_M + WALL_M;

    // Ground from the bottom of the ground band to the floor, with turf on top. Nothing needs
    // clearing above: the floor is above the highest ground, so what is there is air.
    let floor: Vec<Vec<Command>> = strips(region(
        (x.0 - 1.0, SEA_LEVEL_M, z.0 - 1.0),
        (x.1 + 1.0, FLOOR_M, z.1 + 1.0),
    ))
    .into_iter()
    .map(|strip| {
        let layer = |low: f32, high: f32, material| {
            let mut layer = strip;
            layer.min.y = low;
            layer.max.y = high;
            fill(layer, s, material)
        };
        let mut piece = layer(SEA_LEVEL_M, FLOOR_M - 1.0, cube(ids.soil));
        piece.extend(layer(FLOOR_M - 1.0, FLOOR_M, cube(ids.turf)));
        piece
    })
    .collect();

    let wall = |min: (f32, f32), max: (f32, f32)| {
        fill(
            region((min.0, FLOOR_M, min.1), (max.0, top, max.1)),
            s,
            cube(ids.glass),
        )
    };
    let mut south = wall((x.0, z.0), (x.1, z.0 + 1.0));
    south.extend(fill(
        region((DOOR_X_M.0, FLOOR_M, z.0), (DOOR_X_M.1, top, z.0 + 1.0)),
        s,
        cube(0),
    ));
    // A stair up to the path, where the door was cut.
    south.extend(fill(
        region((-1.0, FLOOR_M, z.0), (1.0, FLOOR_M + s, z.0 + 1.0)),
        s,
        voxel(ids.stone, ids.stair),
    ));
    let mut standing = vec![
        south,
        wall((x.0, z.1 - 1.0), (x.1, z.1)),
        wall((x.0, z.0), (x.0 + 1.0, z.1)),
        wall((x.1 - 1.0, z.0), (x.1, z.1)),
        // A path of slabs from the door to the middle.
        fill(
            region((-1.0, FLOOR_M, z.0 + 1.0), (1.0, FLOOR_M + s, z.0 + 12.0)),
            s,
            voxel(ids.stone, ids.slab),
        ),
    ];
    // Lantern posts: stone in the fence Shape, with nothing yet where the lantern will go.
    for &(px, pz) in &LANTERNS_AT_M {
        standing.push(fill(
            region(
                (px, FLOOR_M, pz),
                (px + 1.0, FLOOR_M + POST_HEIGHT_M, pz + 1.0),
            ),
            s,
            voxel(ids.stone, ids.fence),
        ));
    }

    let roof: Vec<Vec<Command>> = strips(region((x.0, top, z.0), (x.1, top + 1.0, z.1)))
        .into_iter()
        .map(|strip| fill(strip, s, cube(ids.frosted)))
        .collect();

    let mut commands = nearest_first(floor);
    commands.extend(nearest_first(standing));
    commands.extend(nearest_first(roof));
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
            survival: None,
            ores: vec![OreDef {
                material: "crystals:amethyst".into(),
                min_depth_m: 8.0,
                max_depth_m: 40.0,
                vein_size_m: 3.0,
                rarity: 0.25,
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
