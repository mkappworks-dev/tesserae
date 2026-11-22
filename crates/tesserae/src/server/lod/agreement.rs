//! How far each coarse level's ground strays from level 0's on Terra, column by column.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tesserae_core::climate::SurfacePlantKind;
use tesserae_core::worldgen::Worldgen;
use tesserae_core::{CHUNK_SIZE, Chunk, MaterialId, Tables, Voxel};

use super::{LodGenerators, level_scale, make_coarse};
use crate::server::lod_rings::level_count;
use crate::server::mods::runtime::tests::terra_loaded;
use crate::server::streaming::streaming_settings;
use crate::world::{ChunkCoord, WorldSettings};

const N: i32 = CHUNK_SIZE as i32;

/// A window is this many level-0 Chunk columns a side: 256 m, so it shares its level-0 Chunks
/// across levels and holds at least 64 level-5 columns.
const WINDOW_CHUNKS: i32 = 8;

const SEARCH_STEP_M: f32 = 64.0;
const SEARCH_RINGS: i32 = 48;

#[derive(Default)]
struct Rates {
    columns: usize,
    level_zero_wet: usize,
    wet_differs: usize,
    /// Columns whose level-0 centre is wet or dry unlike a neighbouring column's.
    coastal: usize,
    coastal_dried: usize,
    top_differs: usize,
    level_zero_trees: usize,
    /// Columns a cave or shaft opens at their level-0 centre, which no coarse level draws, so
    /// they are not compared.
    carved: usize,
}

impl Rates {
    fn share(&self, n: usize) -> f64 {
        100.0 * n as f64 / self.columns as f64
    }
}

/// What a column shows from above, beneath any tree.
#[derive(Clone, Copy, PartialEq)]
struct Ground {
    wet: bool,
    top: MaterialId,
    /// The global Voxel row of `top`.
    y: i32,
}

struct Terra {
    world: WorldSettings,
    full: Worldgen,
    generators: LodGenerators,
    tables: Tables,
    water: MaterialId,
    tree_parts: HashSet<MaterialId>,
    made: HashMap<(u32, ChunkCoord), Arc<Chunk>>,
    made_full: HashMap<ChunkCoord, Arc<Chunk>>,
}

impl Terra {
    fn load() -> Self {
        let loaded = terra_loaded(1.0);
        let world = WorldSettings::default();
        let params = loaded.world.expect("Terra declares its World");
        let full = Worldgen::new(world.seed, world.voxel_size, params)
            .with_templates(&loaded.tables.templates)
            .with_settlements(&loaded.tables.settlements)
            .with_loot(&loaded.biome_loot);
        let levels = level_count(&streaming_settings(loaded.view));
        let generators = LodGenerators::new(&world, &full, levels);
        let tree_parts = params
            .biomes
            .as_slice()
            .iter()
            .flat_map(|b| b.surface_features.as_slice())
            .filter_map(|f| match f.plant?.kind {
                SurfacePlantKind::Tree(t) => Some([t.trunk, t.canopy]),
                _ => None,
            })
            .flatten()
            .collect();
        Terra {
            world,
            full,
            generators,
            tables: loaded.tables,
            water: params.water.expect("Terra has a sea").material,
            tree_parts,
            made: HashMap::new(),
            made_full: HashMap::new(),
        }
    }

    /// The Chunk an untouched level-`level` coord holds; level 0 is the bare terrain level 1
    /// downsamples, the ground beneath every tree and Structure.
    fn chunk(&mut self, level: u32, coord: ChunkCoord) -> Arc<Chunk> {
        if let Some(c) = self.made.get(&(level, coord)) {
            return Arc::clone(c);
        }
        let chunk = if level == 0 {
            self.generators.sources[0].chunk(coord.x, coord.y, coord.z)
        } else {
            let makers = self.generators.makers(level);
            make_coarse(level, coord, makers, &Default::default(), &self.tables).chunk
        };
        let chunk = Arc::new(chunk);
        self.made.insert((level, coord), Arc::clone(&chunk));
        chunk
    }

    fn full_chunk(&mut self, coord: ChunkCoord) -> Arc<Chunk> {
        let full = &self.full;
        Arc::clone(
            self.made_full
                .entry(coord)
                .or_insert_with(|| Arc::new(full.chunk(coord.x, coord.y, coord.z))),
        )
    }

    /// The Chunk row holding the highest anything stands over a level-`level` column, or above.
    fn top_cy(&self, level: u32, [gx, gz]: [i32; 2]) -> i32 {
        let voxel_m = self.world.voxel_size * level_scale(level);
        let min = [gx as f32 * voxel_m, gz as f32 * voxel_m];
        let clear_m = self
            .full
            .clear_height_m(min, [min[0] + voxel_m, min[1] + voxel_m]);
        ((clear_m + voxel_m) / (voxel_m * N as f32)).floor() as i32
    }

    fn ground(&mut self, level: u32, column: [i32; 2]) -> Ground {
        let mut wet = false;
        let from = self.top_cy(level, column);
        let water = self.water;
        let (top, y) = scan(
            |c| self.chunk(level, c),
            column,
            from,
            |v| {
                wet |= v.material() == water;
                !v.is_air() && v.material() != water
            },
        );
        Ground {
            wet,
            top: top.material(),
            y,
        }
    }

    fn stands_a_tree(&mut self, column: [i32; 2]) -> bool {
        let from = self.top_cy(0, column);
        let parts = self.tree_parts.clone();
        let mut tree = false;
        scan(
            |c| self.full_chunk(c),
            column,
            from,
            |v| {
                tree |= parts.contains(&v.material());
                !v.is_air() && !parts.contains(&v.material())
            },
        );
        tree
    }

    fn wet(&self, [x, z]: [f32; 2]) -> bool {
        self.full.water_surface_m(x, z).is_some()
    }

    fn wooded(&self, [x, z]: [f32; 2]) -> bool {
        self.full
            .biome_at(x, z)
            .and_then(|id| self.full.biomes().get(id))
            .is_some_and(|b| {
                b.surface_features
                    .as_slice()
                    .iter()
                    .any(|f| matches!(f.plant.map(|p| p.kind), Some(SurfacePlantKind::Tree(_))))
            })
    }

    /// The nearest point to spawn on a 64 m lattice that `suits`, as the window's middle.
    fn find(&self, suits: impl Fn(&Self, [f32; 2]) -> bool) -> [i32; 2] {
        let [sx, sz] = self.full.spawn_m();
        for r in 0..=SEARCH_RINGS {
            for dx in -r..=r {
                for dz in -r..=r {
                    if dx.abs().max(dz.abs()) != r {
                        continue;
                    }
                    let p = [
                        sx + dx as f32 * SEARCH_STEP_M,
                        sz + dz as f32 * SEARCH_STEP_M,
                    ];
                    if suits(self, p) {
                        return p.map(|m| (m / N as f32).floor() as i32 - WINDOW_CHUNKS / 2);
                    }
                }
            }
        }
        panic!("nothing within {SEARCH_RINGS} rings of spawn suits");
    }

    /// Every level-`level` column whose centre falls in the window at `[cx, cz]`, against the
    /// level-0 column under that centre.
    fn rates(&mut self, level: u32, [cx, cz]: [i32; 2]) -> Rates {
        let span = 1i32 << level;
        let per_chunk = N / span;
        let mut rates = Rates::default();
        for x in 0..WINDOW_CHUNKS * per_chunk {
            for z in 0..WINDOW_CHUNKS * per_chunk {
                let coarse = [cx * per_chunk + x, cz * per_chunk + z];
                let centre = coarse.map(|g| g * span + span / 2);
                let fine = self.ground(0, centre);
                let level_zero = &self.generators.sources[0];
                let surface_m = level_zero
                    .column_top(centre[0], centre[1])
                    .expect("Terra has ground")
                    .surface_m;
                if fine.y < (surface_m / level_zero.voxel_size()).floor() as i32 {
                    rates.carved += 1;
                    continue;
                }
                let seen = self.ground(level, coarse);
                let coastal =
                    [[span, 0], [-span, 0], [0, span], [0, -span]]
                        .iter()
                        .any(|[dx, dz]| {
                            self.ground(0, [centre[0] + dx, centre[1] + dz]).wet != fine.wet
                        });
                rates.coastal += usize::from(coastal);
                rates.coastal_dried += usize::from(coastal && fine.wet && !seen.wet);
                rates.columns += 1;
                rates.level_zero_wet += usize::from(fine.wet);
                rates.wet_differs += usize::from(fine.wet != seen.wet);
                rates.top_differs += usize::from(fine.top != seen.top);
                rates.level_zero_trees += usize::from(self.stands_a_tree(centre));
            }
        }
        rates
    }
}

/// The topmost Voxel that `stops` a downward scan of the column, from Chunk row `from_cy`.
fn scan(
    mut chunk: impl FnMut(ChunkCoord) -> Arc<Chunk>,
    [gx, gz]: [i32; 2],
    from_cy: i32,
    mut stops: impl FnMut(Voxel) -> bool,
) -> (Voxel, i32) {
    let (cx, cz) = (gx.div_euclid(N), gz.div_euclid(N));
    let (x, z) = (gx.rem_euclid(N) as usize, gz.rem_euclid(N) as usize);
    for cy in (from_cy - 64..=from_cy).rev() {
        let c = chunk(ChunkCoord::new(cx, cy, cz));
        if c.uniform() == Some(Voxel::AIR) {
            continue;
        }
        for y in (0..CHUNK_SIZE).rev() {
            let v = c.get(x, y, z);
            if stops(v) {
                return (v, cy * N + y as i32);
            }
        }
    }
    panic!("column {gx},{gz} has no ground below Chunk row {from_cy}");
}

/// Water or land and the top Material are exact at every level wherever no cave opens level 0;
/// run with `--nocapture` to read the rates.
#[test]
fn coarse_ground_agrees_with_level_zero_at_each_column_centre_over_terra_coast_and_forest() {
    let mut terra = Terra::load();
    let step = SEARCH_STEP_M;
    let coast = terra.find(|t, [x, z]| t.wet([x, z]) != t.wet([x + step, z]));
    let forest = terra.find(|t, [x, z]| {
        [
            [0.0, 0.0],
            [step, 0.0],
            [0.0, step],
            [-step, 0.0],
            [0.0, -step],
        ]
        .iter()
        .all(|[dx, dz]| !t.wet([x + dx, z + dz]) && t.wooded([x + dx, z + dz]))
    });
    for (name, window) in [("coast", coast), ("forest", forest)] {
        println!(
            "{name} window: level-0 Chunk columns {window:?} + {WINDOW_CHUNKS} (seed {})",
            terra.world.seed
        );
        println!(
            "level  columns  level 0 wet  wet/dry differs  coastal  coastal dry, level 0 wet  \
             top Material differs  level 0 stands a tree  level 0 carved"
        );
        for level in 1..terra.generators.levels() {
            let r = terra.rates(level, window);
            println!(
                "{level:>5}  {:>7}  {:>10.1}%  {:>14.1}%  {:>7}  {:>23.1}%  {:>19.1}%  {:>20.1}%  \
                 {:>13.1}%",
                r.columns,
                r.share(r.level_zero_wet),
                r.share(r.wet_differs),
                r.coastal,
                100.0 * r.coastal_dried as f64 / r.coastal.max(1) as f64,
                r.share(r.top_differs),
                r.share(r.level_zero_trees),
                100.0 * r.carved as f64 / (r.columns + r.carved) as f64,
            );
            assert_eq!(
                (r.wet_differs, r.top_differs),
                (0, 0),
                "level {level} over the {name} window: columns whose water or land, and top \
                 Material, differ from level 0's"
            );
            match name {
                "coast" => assert!(
                    (1..r.columns).contains(&r.level_zero_wet),
                    "the coast window holds no coast"
                ),
                _ => assert!(r.level_zero_trees > 0, "the forest window stands no trees"),
            }
        }
    }
}
