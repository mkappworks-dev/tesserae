//! Where a coarse generate job's worker CPU goes, per level, over the load
//! extent the Server generates after the `teleport` fixture's jump: 2D height,
//! 3D caves, smoothing, the cube fill, the uniform early-outs and the
//! downsample, and how much of it lies where the finer level draws or outside
//! the mesh shell. Thread CPU, interleaved rounds, medians.
//!
//! `cargo bench -p tesserae-core --bench coarse_breakdown [-- rounds]`
//!
//! A stage is split off by difference: the same Chunks generated without caves,
//! then without smoothing too. The 2D height is timed by sampling the same
//! columns `Worldgen::chunk` samples. The fill is what remains.

use std::hint::black_box;
use std::time::Duration;

use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{CHUNK_SIZE, Chunk, ShapeTable, Worldgen, WorldgenParams, downsample};

/// The demo's World: `World::default`'s seed, a 1 m Voxel, and the demo Game's
/// relief and lake (`games/demo/src/lib.rs`) over the engine's defaults.
const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SEA_LEVEL_M: f32 = 24.0;
const RELIEF_M: f32 = 24.0;

/// Where `teleport` puts the Eye: `profile_fixture teleport ... to=` in its log.
const EYE_M: [f32; 3] = [2000.0, 56.843224, 0.0];

/// The demo's `StreamingSettings`, from the level-1 counts in `frames.flow.csv`
/// (1,639 held, 564 meshed) and `lod_rings.rs`.
const VIEW_M: f32 = 160.0;
const VERTICAL_M: f32 = 96.0;
const HYSTERESIS_M: f32 = 32.0;
const LOD_M: f32 = 4096.0;
const LEVELS: u32 = 5;

fn params() -> WorldgenParams {
    let f = fixtures();
    WorldgenParams {
        sea_level_m: SEA_LEVEL_M,
        relief_m: RELIEF_M,
        water: Some(WaterParams {
            level_m: SEA_LEVEL_M + RELIEF_M * 5.0 / 12.0,
            material: f.water,
        }),
        ..tesserae_core::testgen::world_params()
    }
}

/// `LodGenerators::new`: level 1 reads level 0's generator, the rest skip
/// smoothing.
fn source(level: u32, params: WorldgenParams) -> Worldgen {
    let params = if level == 1 {
        params
    } else {
        WorldgenParams {
            smoothing: None,
            ..params
        }
    };
    Worldgen::new(SEED, 2f32.powi(level as i32 - 1), params)
}

#[derive(Clone, Copy)]
struct Extent {
    horizontal: i32,
    vertical: i32,
}

impl Extent {
    fn contains(self, d: [i32; 3]) -> bool {
        d[1].abs() <= self.vertical && d[0] * d[0] + d[2] * d[2] <= self.horizontal.pow(2)
    }

    fn grown(self, cells: i32) -> Extent {
        Extent {
            horizontal: self.horizontal + cells,
            vertical: self.vertical + cells,
        }
    }

    fn offsets(self) -> Vec<[i32; 3]> {
        let (h, v) = (self.horizontal, self.vertical);
        (-v..=v)
            .flat_map(|y| (-h..=h).flat_map(move |z| (-h..=h).map(move |x| [x, y, z])))
            .filter(|&d| self.contains(d))
            .collect()
    }
}

fn cell_m(level: u32) -> f32 {
    CHUNK_SIZE as f32 * 2f32.powi(level as i32)
}

fn cells(metres: f32, level: u32) -> i32 {
    ((metres / cell_m(level)).ceil() as i32).max(1)
}

/// `lod_rings::outer_extent`.
fn outer(level: u32) -> Extent {
    let scale = 2f32.powi(level as i32);
    let reach = if level == 0 {
        VIEW_M
    } else {
        (VIEW_M * scale).min(LOD_M)
    };
    Extent {
        horizontal: cells(reach, level),
        vertical: cells(VERTICAL_M * scale, level),
    }
}

/// `LevelRings::new`'s load extent.
fn load(level: u32) -> Extent {
    outer(level).grown(cells(HYSTERESIS_M * 2f32.powi(level as i32), level) + 1)
}

fn anchor(level: u32) -> [i32; 3] {
    EYE_M.map(|m| (m / cell_m(level)).floor() as i32)
}

fn child(c: [i32; 3], i: usize) -> [i32; 3] {
    [
        2 * c[0] + (i & 1) as i32,
        2 * c[1] + (i >> 1 & 1) as i32,
        2 * c[2] + (i >> 2) as i32,
    ]
}

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

unsafe extern "C" {
    fn clock_gettime(clock: u32, ts: *mut Timespec) -> i32;
}

#[cfg(target_os = "macos")]
const CLOCK_THREAD_CPUTIME_ID: u32 = 16;
#[cfg(not(target_os = "macos"))]
const CLOCK_THREAD_CPUTIME_ID: u32 = 3;

fn thread_cpu() -> Duration {
    let mut ts = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid out-pointer for the call's duration.
    unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

fn cpu_ms(f: impl FnOnce()) -> f64 {
    let start = thread_cpu();
    f();
    (thread_cpu() - start).as_secs_f64() * 1e3
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Above the surface band: `Chunk::air` before any noise.
    Air,
    /// Below the soil, with no cave possible in it.
    Rock,
    /// Below the soil, carved where a tunnel could be.
    RockCarved,
    /// Crossed by the surface band: the full path.
    Surface,
}

fn kind(source: &Worldgen, p: WorldgenParams, c: [i32; 3]) -> Kind {
    let v = source.voxel_size();
    let size = CHUNK_SIZE as f32;
    let floor_m = c[1] as f32 * size * v;
    let ceiling_m = floor_m + (size - 1.0) * v;
    let (lowest, highest) = source.surface_bounds_m();
    if floor_m > highest {
        Kind::Air
    } else if ceiling_m < lowest - p.soil_depth_m {
        // `may_carve` is private; a Chunk it rules out stays uniform, and one it
        // lets through is carved, so `uniform` is the closest outside test.
        let caves = p.caves.is_some_and(|caves| ceiling_m >= caves.floor_m);
        if caves { Kind::RockCarved } else { Kind::Rock }
    } else {
        Kind::Surface
    }
}

/// Where a coarse Chunk lies in its level's rings.
#[derive(Clone, Copy, PartialEq)]
enum Region {
    /// Every child inside the finer level's mesh extent: drawn by it.
    Finer,
    /// Inside this level's mesh extent and not wholly under the finer level.
    Shell,
    /// Generated for hysteresis and neighbours, never meshed here.
    Margin,
}

struct Job {
    coord: [i32; 3],
    region: Region,
    kinds: [Kind; 8],
}

fn jobs(level: u32, p: WorldgenParams) -> Vec<Job> {
    let source = source(level, p);
    let (a, fa) = (anchor(level), anchor(level - 1));
    load(level)
        .offsets()
        .into_iter()
        .map(|d| {
            let coord = [a[0] + d[0], a[1] + d[1], a[2] + d[2]];
            let finer = (0..8).all(|i| {
                let c = child(coord, i);
                outer(level - 1).contains([c[0] - fa[0], c[1] - fa[1], c[2] - fa[2]])
            });
            let region = if finer {
                Region::Finer
            } else if outer(level).contains(d) {
                Region::Shell
            } else {
                Region::Margin
            };
            let kinds = std::array::from_fn(|i| kind(&source, p, child(coord, i)));
            Job {
                coord,
                region,
                kinds,
            }
        })
        .collect()
}

/// One round's thread CPU, in ms, for one level.
#[derive(Default, Clone, Copy)]
struct Round {
    air: f64,
    rock: f64,
    rock_carved: f64,
    surface: f64,
    surface_no_caves: f64,
    surface_cubes: f64,
    height: f64,
    downsample: f64,
    finer: f64,
    margin: f64,
}

fn generate(source: &Worldgen, jobs: &[Job], want: Kind) -> f64 {
    cpu_ms(|| {
        for job in jobs {
            for (i, &k) in job.kinds.iter().enumerate() {
                if k == want {
                    let [x, y, z] = child(job.coord, i);
                    black_box(source.chunk(x, y, z));
                }
            }
        }
    })
}

fn whole(source: &Worldgen, shapes: &ShapeTable, jobs: &[Job], region: Region) -> f64 {
    cpu_ms(|| {
        for job in jobs.iter().filter(|j| j.region == region) {
            let children: [Chunk; 8] = std::array::from_fn(|i| {
                let [x, y, z] = child(job.coord, i);
                source.chunk(x, y, z)
            });
            black_box(downsample(std::array::from_fn(|i| &children[i]), shapes));
        }
    })
}

fn round(level: u32, p: WorldgenParams, jobs: &[Job], children: &[[Chunk; 8]]) -> Round {
    let shapes = &fixtures().tables.shapes;
    let full = source(level, p);
    let no_caves = source(level, WorldgenParams { caves: None, ..p });
    let cubes = source(
        level,
        WorldgenParams {
            caves: None,
            smoothing: None,
            ..p
        },
    );
    let reach = if level == 1 && p.smoothing.is_some() {
        1
    } else {
        0
    };
    let span = -reach..CHUNK_SIZE as i32 + reach;
    let v = full.voxel_size();
    let height = cpu_ms(|| {
        for job in jobs {
            for (i, &k) in job.kinds.iter().enumerate() {
                if k == Kind::Surface {
                    let [cx, _, cz] = child(job.coord, i);
                    for z in span.clone() {
                        for x in span.clone() {
                            black_box(full.surface_height_m(
                                (cx * CHUNK_SIZE as i32 + x) as f32 * v,
                                (cz * CHUNK_SIZE as i32 + z) as f32 * v,
                            ));
                        }
                    }
                }
            }
        }
    });
    Round {
        air: generate(&full, jobs, Kind::Air),
        rock: generate(&full, jobs, Kind::Rock),
        rock_carved: generate(&full, jobs, Kind::RockCarved),
        surface: generate(&full, jobs, Kind::Surface),
        surface_no_caves: generate(&no_caves, jobs, Kind::Surface),
        surface_cubes: generate(&cubes, jobs, Kind::Surface),
        height,
        downsample: cpu_ms(|| {
            for c in children {
                black_box(downsample(std::array::from_fn(|i| &c[i]), shapes));
            }
        }),
        finer: whole(&full, shapes, jobs, Region::Finer),
        margin: whole(&full, shapes, jobs, Region::Margin),
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let rounds: usize = std::env::args()
        .skip(1)
        .find_map(|a| a.parse().ok())
        .unwrap_or(3);
    let p = params();
    let levels: Vec<(u32, Vec<Job>, Vec<[Chunk; 8]>)> = (1..=LEVELS)
        .map(|level| {
            let jobs = jobs(level, p);
            let source = source(level, p);
            let children = jobs
                .iter()
                .map(|j| {
                    std::array::from_fn(|i| {
                        let [x, y, z] = child(j.coord, i);
                        source.chunk(x, y, z)
                    })
                })
                .collect();
            (level, jobs, children)
        })
        .collect();
    let mut all: Vec<Vec<Round>> = vec![Vec::new(); levels.len()];
    for _ in 0..rounds {
        for (n, (level, jobs, children)) in levels.iter().enumerate() {
            all[n].push(round(*level, p, jobs, children));
        }
    }
    println!(
        "level jobs finer margin | children air rock carved surface | uniform-in | \
         ms: total early(air rock carved) surface(height caves smooth fill) downsample | \
         finer% margin%"
    );
    for (n, (level, jobs, children)) in levels.iter().enumerate() {
        let m = |f: fn(&Round) -> f64| median(all[n].iter().map(f).collect());
        let count = |k: Kind| jobs.iter().flat_map(|j| j.kinds).filter(|&x| x == k).count();
        let regions = |r: Region| jobs.iter().filter(|j| j.region == r).count();
        let uniform = children
            .iter()
            .filter(|c| c[0].uniform().is_some() && c.iter().all(|x| x.uniform() == c[0].uniform()))
            .count();
        let (air, rock, carved, surface) = (
            m(|r| r.air),
            m(|r| r.rock),
            m(|r| r.rock_carved),
            m(|r| r.surface),
        );
        let caves = surface - m(|r| r.surface_no_caves);
        let smooth = m(|r| r.surface_no_caves) - m(|r| r.surface_cubes);
        let height = m(|r| r.height);
        let fill = m(|r| r.surface_cubes) - height;
        let down = m(|r| r.downsample);
        let total = air + rock + carved + surface + down;
        println!(
            "L{level} {:5} {:4} {:5} | {:5} {:5} {:5} {:5} {:5} | {:5} | \
             {:7.1} {:6.2} {:6.2} {:6.1} {:7.1} ({:6.1} {:6.1} {:6.1} {:6.1}) {:6.1} | {:4.1} {:4.1}",
            jobs.len(),
            regions(Region::Finer),
            regions(Region::Margin),
            jobs.len() * 8,
            count(Kind::Air),
            count(Kind::Rock),
            count(Kind::RockCarved),
            count(Kind::Surface),
            uniform,
            total,
            air,
            rock,
            carved,
            surface,
            height,
            caves,
            smooth,
            fill,
            down,
            100.0 * m(|r| r.finer) / total,
            100.0 * m(|r| r.margin) / total,
        );
    }
}
