//! How far each coarse level's ground strays from level 0's on Terra, column by column.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use tesserae_core::climate::SurfacePlantKind;
use tesserae_core::worldgen::Worldgen;
use tesserae_core::{CHUNK_SIZE, Chunk, MaterialId, Tables, Voxel};

use super::{
    Held, IMPOSTORS_TO, LodGenerators, Makers, Shown, face_neighbors, level_scale, make_coarse,
    mesh_coarse,
};
use crate::server::lod_rings::level_count;
use crate::server::mods::runtime::tests::terra_loaded;
use crate::server::streaming::declared_streaming;
use crate::world::{ChunkCoord, WorldSettings};

const N: i32 = CHUNK_SIZE as i32;

/// A window is this many level-0 Chunk columns a side: 256 m, so it shares its level-0 Chunks
/// across levels and holds at least 64 level-5 columns.
const WINDOW_CHUNKS: i32 = 8;

const SEARCH_STEP_M: f32 = 64.0;
const CROSS: [[f32; 2]; 5] = [
    [0.0, 0.0],
    [SEARCH_STEP_M, 0.0],
    [0.0, SEARCH_STEP_M],
    [-SEARCH_STEP_M, 0.0],
    [0.0, -SEARCH_STEP_M],
];
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
    /// Columns any level-0 column of whose footprint stands a tree, and those the coarse level
    /// stands one on, from level 1 to [`IMPOSTORS_TO`].
    footprint_trees: usize,
    stood_trees: usize,
    trees_missing: usize,
    trees_extra: usize,
    /// Above [`IMPOSTORS_TO`]: columns over half of whose footprint level 0 stands a tree under,
    /// those the coarse level draws Canopy cover on, those that differ, and those that differ from
    /// every column within one of them.
    dense_trees: usize,
    canopy: usize,
    canopy_differs: usize,
    canopy_off_edge: usize,
    /// Columns Canopy cover stands on that are wet at their level-0 centre.
    canopy_wet: usize,
    /// Canopy cover's columns by how many Voxels its top stands over the ground, and by its leaves.
    canopy_rises: BTreeMap<i32, usize>,
    canopy_leaves: BTreeMap<MaterialId, usize>,
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
    built: HashSet<MaterialId>,
    made: HashMap<(u32, ChunkCoord), Arc<Chunk>>,
    stood: HashMap<(u32, ChunkCoord), Arc<Chunk>>,
    made_full: HashMap<ChunkCoord, Arc<Chunk>>,
    under_tree: HashMap<[i32; 2], bool>,
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
        let levels = level_count(&declared_streaming(loaded.view).low());
        let generators = LodGenerators::new(&world, &full, levels);
        let tree_parts = params
            .biomes
            .as_slice()
            .iter()
            .flat_map(|b| b.surface_features.as_slice())
            .filter_map(|f| match f.plant?.kind {
                SurfacePlantKind::Tree(t) => Some(
                    std::iter::once(t.trunk)
                        .chain(t.canopies(params.biomes.variety()).as_slice().to_vec()),
                ),
                _ => None,
            })
            .flatten()
            .collect();
        let built = generators.built.iter().copied().collect();
        Terra {
            world,
            full,
            generators,
            tables: loaded.tables,
            water: params.water.expect("Terra has a sea").material,
            tree_parts,
            built,
            made: HashMap::new(),
            stood: HashMap::new(),
            made_full: HashMap::new(),
            under_tree: HashMap::new(),
        }
    }

    /// The ground of the Chunk an untouched level-`level` coord holds, beneath its Impostors;
    /// level 0 is the bare terrain level 1 downsamples, the ground beneath every tree and
    /// Structure.
    fn chunk(&mut self, level: u32, coord: ChunkCoord) -> Arc<Chunk> {
        if let Some(c) = self.made.get(&(level, coord)) {
            return Arc::clone(c);
        }
        let chunk = if level == 0 {
            self.generators.sources[0].chunk(coord.x, coord.y, coord.z)
        } else {
            let makers = Makers {
                stand_trees: false,
                stand_structures: false,
                ..self.generators.makers(level)
            };
            make_coarse(level, coord, makers, &Default::default(), &self.tables).chunk
        };
        let chunk = Arc::new(chunk);
        self.made.insert((level, coord), Arc::clone(&chunk));
        chunk
    }

    /// The Chunk an untouched level-`level` coord holds, Impostors and all.
    fn stood_chunk(&mut self, level: u32, coord: ChunkCoord) -> Arc<Chunk> {
        if let Some(c) = self.stood.get(&(level, coord)) {
            return Arc::clone(c);
        }
        let makers = self.generators.makers(level);
        let chunk = make_coarse(level, coord, makers, &Default::default(), &self.tables).chunk;
        let chunk = Arc::new(chunk);
        self.stood.insert((level, coord), Arc::clone(&chunk));
        chunk
    }

    fn stands_an_impostor(&mut self, level: u32, column: [i32; 2]) -> bool {
        let from = self.top_cy(level, column);
        let parts = self.tree_parts.clone();
        let mut tree = false;
        scan(
            |c| self.stood_chunk(level, c),
            column,
            from,
            |v| {
                tree |= parts.contains(&v.material());
                !v.is_air() && !parts.contains(&v.material())
            },
        );
        tree
    }

    /// The row and Material of a column's topmost Voxel at `level`, if a tree's.
    fn tree_top(&mut self, level: u32, column: [i32; 2]) -> Option<(i32, MaterialId)> {
        let from = self.top_cy(level, column);
        let (v, y) = scan(
            |c| self.stood_chunk(level, c),
            column,
            from,
            |v| !v.is_air(),
        );
        self.tree_parts
            .contains(&v.material())
            .then_some((y, v.material()))
    }

    /// Whether a solid Structure Voxel stands over the column's ground at `level`, level 0 read
    /// whole, and the ground's top Material beneath any Impostor.
    fn built_over(&mut self, level: u32, column: [i32; 2]) -> (bool, MaterialId) {
        let from = self.top_cy(level, column);
        let (trees, built) = (self.tree_parts.clone(), self.built.clone());
        let materials = self.tables.materials.clone();
        let full = self.full.clone();
        let mut stands = false;
        let mut stops = |v: Voxel, y: i32| {
            let m = v.material();
            if built.contains(&m) && materials.solid(v) {
                // Level 0 grows some of a field's crops outside any Structure.
                stands |= level > 0 || full.stood_at([column[0], y, column[1]]).is_some();
            }
            !v.is_air() && !trees.contains(&m) && !built.contains(&m)
        };
        let (top, _) = if level == 0 {
            scan_at(|c| self.full_chunk(c), column, from, &mut stops)
        } else {
            scan_at(|c| self.stood_chunk(level, c), column, from, &mut stops)
        };
        (stands, top.material())
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
        if let Some(&tree) = self.under_tree.get(&column) {
            return tree;
        }
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
        self.under_tree.insert(column, tree);
        tree
    }

    /// Whether level 0 stands a tree over more than half of a level-`level` column's footprint.
    fn densely_wooded(&mut self, level: u32, coarse: [i32; 2]) -> bool {
        let span = 1i32 << level;
        let under = (0..span * span)
            .filter(|i| {
                self.stands_a_tree([coarse[0] * span + i % span, coarse[1] * span + i / span])
            })
            .count();
        2 * under > (span * span) as usize
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

    /// Dry, wooded ground round a window's middle.
    fn forest_window(&self) -> [i32; 2] {
        self.find(|t, [x, z]| {
            CROSS
                .iter()
                .all(|[dx, dz]| !t.wet([x + dx, z + dz]) && t.wooded([x + dx, z + dz]))
        })
    }

    /// Dry ground Canopy cover stands on round a window's middle.
    fn canopy_window(&self) -> [i32; 2] {
        self.find(|t, [x, z]| {
            let p = &t.generators.placement;
            let columns =
                CROSS.map(|[dx, dz]| [x + dx, z + dz].map(|m| (m / p.voxel_size()).floor() as i32));
            !t.wet([x, z])
                && p.canopy_cover(&t.generators.canopies, &columns)
                    .iter()
                    .all(Option::is_some)
        })
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
                if level <= IMPOSTORS_TO {
                    let footprint = (0..span * span).any(|i| {
                        self.stands_a_tree([
                            coarse[0] * span + i % span,
                            coarse[1] * span + i / span,
                        ])
                    });
                    let stood = self.stands_an_impostor(level, coarse);
                    rates.footprint_trees += usize::from(footprint);
                    rates.stood_trees += usize::from(stood);
                    rates.trees_missing += usize::from(footprint && !stood);
                    rates.trees_extra += usize::from(stood && !footprint);
                } else {
                    let dense = self.densely_wooded(level, coarse);
                    let canopy = self.stands_an_impostor(level, coarse);
                    rates.dense_trees += usize::from(dense);
                    rates.canopy += usize::from(canopy);
                    rates.canopy_wet += usize::from(canopy && fine.wet);
                    if let Some((y, leaves)) = self.tree_top(level, coarse).filter(|_| canopy) {
                        *rates.canopy_rises.entry(y - seen.y).or_default() += 1;
                        *rates.canopy_leaves.entry(leaves).or_default() += 1;
                    }
                    if dense != canopy {
                        rates.canopy_differs += 1;
                        let near = (0..9).any(|i| {
                            let at = [coarse[0] + i % 3 - 1, coarse[1] + i / 3 - 1];
                            self.densely_wooded(level, at) == canopy
                        });
                        rates.canopy_off_edge += usize::from(!near);
                    }
                }
            }
        }
        rates
    }
}

/// The topmost Voxel that `stops` a downward scan of the column, from Chunk row `from_cy`.
fn scan(
    chunk: impl FnMut(ChunkCoord) -> Arc<Chunk>,
    column: [i32; 2],
    from_cy: i32,
    mut stops: impl FnMut(Voxel) -> bool,
) -> (Voxel, i32) {
    scan_at(chunk, column, from_cy, |v, _| stops(v))
}

/// [`scan`], `stops` also told each Voxel's global row.
fn scan_at(
    mut chunk: impl FnMut(ChunkCoord) -> Arc<Chunk>,
    [gx, gz]: [i32; 2],
    from_cy: i32,
    mut stops: impl FnMut(Voxel, i32) -> bool,
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
            let g = cy * N + y as i32;
            if stops(v, g) {
                return (v, g);
            }
        }
    }
    panic!("column {gx},{gz} has no ground below Chunk row {from_cy}");
}

/// Water or land and the top Material are exact at every level wherever no cave opens level 0,
/// every footprint level 0 stands a tree in stands one at levels 1 to [`IMPOSTORS_TO`], and above
/// them Canopy cover keeps off water, follows level 0's tree density to within one column, and
/// stands a dense forest's top at more than one height up to level 4; run with `--nocapture` to
/// read the rates.
#[test]
fn coarse_ground_agrees_with_level_zero_at_each_column_centre_over_terra_coast_and_forest() {
    let mut terra = Terra::load();
    let step = SEARCH_STEP_M;
    let coast = terra.find(|t, [x, z]| t.wet([x, z]) != t.wet([x + step, z]));
    let forest = terra.forest_window();
    let canopied = terra.canopy_window();
    for (name, window) in [("coast", coast), ("forest", forest), ("canopy", canopied)] {
        println!(
            "{name} window: level-0 Chunk columns {window:?} + {WINDOW_CHUNKS} (seed {})",
            terra.world.seed
        );
        println!(
            "level  columns  level 0 wet  wet/dry differs  coastal  coastal dry, level 0 wet  \
             top Material differs  level 0 stands a tree  level 0 carved"
        );
        let mut trees = Vec::new();
        let mut canopies = Vec::new();
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
            if level <= IMPOSTORS_TO {
                trees.push((
                    level,
                    r.columns,
                    r.footprint_trees,
                    r.stood_trees,
                    r.trees_missing,
                    r.trees_extra,
                ));
            } else {
                assert_eq!(
                    r.canopy_wet, 0,
                    "level {level} over the {name} window: Canopy cover on water"
                );
                canopies.push((
                    level,
                    r.columns,
                    r.dense_trees,
                    r.canopy,
                    r.canopy_differs,
                    r.canopy_off_edge,
                ));
                println!(
                    "level {level} Canopy cover: columns by Voxels over the ground {:?}, by \
                     leaves {:?}",
                    r.canopy_rises, r.canopy_leaves
                );
                if name == "canopy" && level <= 4 {
                    assert!(
                        r.canopy_rises.len() > 1,
                        "level {level}: a dense forest's top stands at one height, {:?}",
                        r.canopy_rises
                    );
                }
            }
            match name {
                "coast" => assert!(
                    (1..r.columns).contains(&r.level_zero_wet),
                    "the coast window holds no coast"
                ),
                _ => assert!(r.level_zero_trees > 0, "the forest window stands no trees"),
            }
        }
        println!("level  columns  level 0 stands a tree in the footprint  stood  missing  extra");
        for (level, columns, footprint, stood, missing, extra) in trees {
            let share = |n: usize| 100.0 * n as f64 / columns as f64;
            println!(
                "{level:>5}  {columns:>7}  {:>36.1}%  {:>4.1}%  {:>6.2}%  {:>4.2}%",
                share(footprint),
                share(stood),
                share(missing),
                share(extra),
            );
            assert!(
                stood > 0,
                "level {level} stands no tree over the {name} window"
            );
            // An extra is a cell level 0 gave to something else first: a tuft, or an overhang.
            assert!(
                missing == 0 && extra * 100 <= columns,
                "level {level} over the {name} window: {missing} columns miss a tree, {extra} \
                 stand one level 0 does not"
            );
        }
        println!("level  columns  level 0 trees over half  Canopy cover  differs  past one column");
        for (level, columns, dense, canopy, differs, off_edge) in canopies {
            let share = |n: usize| 100.0 * n as f64 / columns as f64;
            println!(
                "{level:>5}  {columns:>7}  {:>23.1}%  {:>11.1}%  {:>6.1}%  {:>14.2}%",
                share(dense),
                share(canopy),
                share(differs),
                share(off_edge),
            );
            assert!(
                off_edge <= 1 || off_edge * 50 <= columns,
                "level {level} over the {name} window: {off_edge} columns' Canopy cover differs \
                 from level 0's tree density past one column"
            );
        }
    }
}

/// Over the Settlement Terra spawns in, at levels 1 to 3: each column whose footprint holds a
/// solid Structure Voxel at level 0 stands one, and each column whose centre is a road or trail at
/// level 0 shows its path as its ground's top, where no Structure stands over it. Levels 4 and 5
/// hold the Chunks they would without Structures or paths. Run with `--nocapture` to read the
/// rates.
#[test]
fn levels_one_to_three_stand_terras_spawn_settlement_and_its_paths_as_level_zero_does() {
    let mut terra = Terra::load();
    let window = terra.find(|_, _| true);
    println!(
        "settlement window: level-0 Chunk columns {window:?} + {WINDOW_CHUNKS} (seed {})",
        terra.world.seed
    );
    println!(
        "level  columns  level 0 built in the footprint  stood  missing  extra  paths  differ"
    );
    for level in 1..=3 {
        let span = 1i32 << level;
        let per_chunk = N / span;
        let (mut columns, mut footprint, mut stood, mut missing, mut extra) = (0, 0, 0, 0, 0);
        let (mut paths, mut differ) = (0, 0);
        for x in 0..WINDOW_CHUNKS * per_chunk {
            for z in 0..WINDOW_CHUNKS * per_chunk {
                let coarse = [window[0] * per_chunk + x, window[1] * per_chunk + z];
                let centre = coarse.map(|g| g * span + span / 2);
                columns += 1;
                let built = (0..span * span).any(|i| {
                    terra
                        .built_over(
                            0,
                            [coarse[0] * span + i % span, coarse[1] * span + i / span],
                        )
                        .0
                });
                let (stands, top) = terra.built_over(level, coarse);
                footprint += usize::from(built);
                stood += usize::from(stands);
                missing += usize::from(built && !stands);
                extra += usize::from(stands && !built);
                // A coarse Voxel a Structure reaches is the Structure's, path or not.
                if let Some(path) = terra.generators.placement.paths(&[centre])[0]
                    && !stands
                {
                    paths += 1;
                    differ += usize::from(top != path);
                }
            }
        }
        let share = |n: usize| 100.0 * n as f64 / columns as f64;
        println!(
            "{level:>5}  {columns:>7}  {:>29.1}%  {:>4.1}%  {:>6.2}%  {:>4.2}%  {paths:>5}  \
             {differ:>6}",
            share(footprint),
            share(stood),
            share(missing),
            share(extra),
        );
        assert!(footprint > 0, "level {level}: no Structure in the window");
        assert_eq!(missing, 0, "level {level}: footprints that stand nothing");
        assert!(
            extra * 100 <= columns,
            "level {level}: {extra} columns stand a Structure level 0 does not"
        );
        assert!(level > 1 || paths > 0, "no path in the window");
        assert_eq!(
            differ, 0,
            "level {level}: path columns whose top is not the path"
        );
    }
    for level in 4..terra.generators.levels() {
        let span = 1i32 << level;
        let [first, last] =
            [0, WINDOW_CHUNKS - 1].map(|d| window.map(|w| (w + d).div_euclid(span)));
        let cell_m = N as f32 * terra.world.voxel_size * level_scale(level);
        let (lowest_m, highest_m) = terra.generators.sources[0].surface_bounds_m();
        let rows = (lowest_m / cell_m).floor() as i32 - 1..=(highest_m / cell_m).floor() as i32 + 1;
        let mut stood = 0;
        for cz in first[1]..=last[1] {
            for cx in first[0]..=last[0] {
                for cy in rows.clone() {
                    let coord = ChunkCoord::new(cx, cy, cz);
                    let bare = Makers {
                        stand_structures: false,
                        ..terra.generators.makers(level)
                    };
                    let bare =
                        make_coarse(level, coord, bare, &Default::default(), &terra.tables).chunk;
                    let stood_chunk = terra.stood_chunk(level, coord);
                    let n = CHUNK_SIZE;
                    assert!(
                        (0..n.pow(3)).all(|i| {
                            let [x, y, z] = [i % n, i / n % n, i / n.pow(2)];
                            stood_chunk.get(x, y, z) == bare.get(x, y, z)
                        }),
                        "level {level}: Chunk {coord:?} differs from its Chunk without Structures"
                    );
                    stood += 1;
                }
            }
        }
        assert!(stood > 0, "level {level}: no Chunk compared");
    }
}

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

unsafe extern "C" {
    fn clock_gettime(clock: u32, ts: *mut Timespec) -> i32;
}

/// This thread's CPU seconds, whatever else the host runs.
fn thread_cpu_s() -> f64 {
    const CLOCK_THREAD_CPUTIME_ID: u32 = 16;
    let mut ts = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid out-pointer for the call's duration.
    unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9
}

/// Every Chunk of a `side` x `side` block of level-`level` Chunk columns over the forest window's
/// middle, each row the ground or its trees reach.
fn forest_block(terra: &Terra, forest: [i32; 2], level: u32, side: i32) -> Vec<ChunkCoord> {
    let span = 1i32 << level;
    let middle = forest.map(|c| (c + WINDOW_CHUNKS / 2).div_euclid(span));
    let cell_m = N as f32 * terra.world.voxel_size * level_scale(level);
    let (lowest, highest) = terra.full.surface_bounds_m();
    let rows = (lowest / cell_m).floor() as i32..=(highest / cell_m).floor() as i32;
    let mut block = Vec::new();
    for cy in rows {
        for dz in 0..side {
            for dx in 0..side {
                block.push(ChunkCoord::new(
                    middle[0] - side / 2 + dx,
                    cy,
                    middle[1] - side / 2 + dz,
                ));
            }
        }
    }
    block
}

/// Coarse-Chunk CPU time with and without the trees, Impostors at levels 1 to [`IMPOSTORS_TO`] and
/// Canopy cover above, interleaved in mirrored order with cold caches every pass, and the vertex
/// bytes each meshes to. Run alone in release with `--ignored --nocapture`.
#[test]
#[ignore = "a measurement, not a check"]
fn coarse_chunk_cpu_time_and_vertex_bytes_with_and_without_trees_over_terra_forest() {
    const ROUNDS: usize = 9;
    let terra = Terra::load();
    for forest in [terra.forest_window(), terra.canopy_window()] {
        cpu_time_and_vertex_bytes(&terra, forest, ROUNDS);
    }
}

fn cpu_time_and_vertex_bytes(terra: &Terra, forest: [i32; 2], rounds: usize) {
    let levels = terra.generators.levels();
    let pass = |level: u32, impostors: bool, block: &[ChunkCoord]| -> f64 {
        terra.full.forget_caches();
        let generators = LodGenerators::new(&terra.world, &terra.full, levels);
        let makers = generators.makers(level);
        let makers = Makers {
            stand_trees: impostors,
            ..makers
        };
        let mut spent = 0.0;
        for &coord in block {
            let from = thread_cpu_s();
            std::hint::black_box(make_coarse(
                level,
                coord,
                makers,
                &Default::default(),
                &terra.tables,
            ));
            spent += thread_cpu_s() - from;
        }
        1000.0 * spent / block.len() as f64
    };
    println!("window {forest:?}, seed {}", terra.world.seed);
    println!("level  chunks  without ms  with ms  with / without");
    for level in 1..levels {
        let block = forest_block(terra, forest, level, 3);
        let (mut without, mut with) = (Vec::new(), Vec::new());
        for round in 0..rounds {
            if round % 2 == 0 {
                without.push(pass(level, false, &block));
                with.push(pass(level, true, &block));
            } else {
                with.push(pass(level, true, &block));
                without.push(pass(level, false, &block));
            }
        }
        let median = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        };
        let (a, b) = (median(&mut without), median(&mut with));
        println!(
            "{level:>5}  {:>6}  {a:>10.2}  {b:>7.2}  {:>14.2}",
            block.len(),
            b / a
        );
    }

    println!("level  meshes  without vertex bytes  with vertex bytes");
    for level in 1..levels {
        let block = forest_block(terra, forest, level, 3);
        let generators = LodGenerators::new(&terra.world, &terra.full, levels);
        let voxel_m = terra.world.voxel_size * level_scale(level);
        let surface = &generators.sources[level as usize - 1];
        let mut bytes = [0usize; 2];
        for (i, impostors) in [false, true].into_iter().enumerate() {
            let mut made: HashMap<ChunkCoord, Shown> = HashMap::new();
            let mut shown = |coord: ChunkCoord| {
                made.entry(coord)
                    .or_insert_with(|| {
                        let makers = generators.makers(level);
                        let makers = Makers {
                            stand_trees: impostors,
                            ..makers
                        };
                        let coarse =
                            make_coarse(level, coord, makers, &Default::default(), &terra.tables);
                        Shown {
                            chunk: Arc::new(coarse.chunk),
                            overlay: Arc::new(coarse.overlay),
                            openings: Arc::new(coarse.openings),
                        }
                    })
                    .clone()
            };
            for &coord in &block {
                let around: [Shown; 6] = face_neighbors(coord).map(&mut shown);
                let own = shown(coord);
                let mesh =
                    mesh_coarse(surface, level, voxel_m, coord, &own, &around, &terra.tables);
                bytes[i] += [&mesh.opaque, &mesh.blended, &mesh.cutout]
                    .iter()
                    .map(|layer| size_of_val(&layer.quads[..]))
                    .sum::<usize>();
            }
        }
        println!(
            "{level:>5}  {:>6}  {:>20}  {:>17}",
            block.len(),
            bytes[0],
            bytes[1]
        );
    }
}

/// What a level-1 job inside level 0's load pays to generate its eight children instead of
/// reading the held ones, Impostors drawn either way: CPU time interleaved in mirrored order,
/// cold coarse caches every pass. Run alone with `--ignored --nocapture`.
#[test]
#[ignore = "a measurement, not a check"]
fn level_one_cpu_time_generating_children_against_reading_held_ones_by_terra_spawn() {
    const ROUNDS: usize = 9;
    let terra = Terra::load();
    let [sx, sz] = terra.full.spawn_m();
    let at = [sx, sz].map(|m| (m / N as f32).floor() as i32 - WINDOW_CHUNKS / 2);
    let block = forest_block(&terra, at, 1, 3);
    let held: HashMap<ChunkCoord, [Option<Held>; 8]> = block
        .iter()
        .map(|&coord| {
            let children = std::array::from_fn(|i| {
                let c = coord.child(i);
                Some(Held {
                    chunk: Arc::new(terra.full.chunk(c.x, c.y, c.z)),
                    changed: None,
                })
            });
            (coord, children)
        })
        .collect();
    let levels = terra.generators.levels();
    let pass = |read: bool| -> f64 {
        terra.full.forget_caches();
        let generators = LodGenerators::new(&terra.world, &terra.full, levels);
        let none = Default::default();
        let mut spent = 0.0;
        for coord in &block {
            let children = if read { &held[coord] } else { &none };
            let from = thread_cpu_s();
            std::hint::black_box(make_coarse(
                1,
                *coord,
                generators.makers(1),
                children,
                &terra.tables,
            ));
            spent += thread_cpu_s() - from;
        }
        1000.0 * spent / block.len() as f64
    };
    let (mut read, mut generated) = (Vec::new(), Vec::new());
    for round in 0..ROUNDS {
        if round % 2 == 0 {
            read.push(pass(true));
            generated.push(pass(false));
        } else {
            generated.push(pass(false));
            read.push(pass(true));
        }
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let (a, b) = (median(&mut read), median(&mut generated));
    println!("spawn block {at:?}, seed {}", terra.world.seed);
    println!("chunks  reading held ms  generating ms  extra ms");
    println!("{:>6}  {a:>15.2}  {b:>13.2}  {:>8.2}", block.len(), b - a);
}

/// Coarse-Chunk CPU time with and without Structure Impostors over Terra's spawn Settlement, at
/// every coarse level, interleaved in mirrored order with cold caches every pass. Run alone in
/// release with `--ignored --nocapture`.
#[test]
#[ignore = "a measurement, not a check"]
fn coarse_chunk_cpu_time_with_and_without_structure_impostors_over_terra_settlement() {
    const ROUNDS: usize = 9;
    let terra = Terra::load();
    let window = terra.find(|_, _| true);
    let levels = terra.generators.levels();
    let pass = |level: u32, structures: bool, block: &[ChunkCoord]| -> f64 {
        terra.full.forget_caches();
        let generators = LodGenerators::new(&terra.world, &terra.full, levels);
        let makers = Makers {
            stand_structures: structures,
            ..generators.makers(level)
        };
        let mut spent = 0.0;
        for &coord in block {
            let from = thread_cpu_s();
            std::hint::black_box(make_coarse(
                level,
                coord,
                makers,
                &Default::default(),
                &terra.tables,
            ));
            spent += thread_cpu_s() - from;
        }
        1000.0 * spent / block.len() as f64
    };
    println!("settlement window {window:?}, seed {}", terra.world.seed);
    println!("level  chunks  without ms  with ms  with / without");
    for level in 1..levels {
        let block = forest_block(&terra, window, level, 3);
        let (mut without, mut with) = (Vec::new(), Vec::new());
        for round in 0..ROUNDS {
            if round % 2 == 0 {
                without.push(pass(level, false, &block));
                with.push(pass(level, true, &block));
            } else {
                with.push(pass(level, true, &block));
                without.push(pass(level, false, &block));
            }
        }
        let median = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        };
        let (a, b) = (median(&mut without), median(&mut with));
        println!(
            "{level:>5}  {:>6}  {a:>10.2}  {b:>7.2}  {:>14.2}",
            block.len(),
            b / a
        );
    }
}
