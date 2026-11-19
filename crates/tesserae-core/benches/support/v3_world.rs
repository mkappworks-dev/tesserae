//! A Generator version 3 World shaped like Terra's, and the Chunks the version 3 bench times, each
//! found by a fixed scan so a figure names the same Chunk on every run and build.

#![allow(dead_code)]

use tesserae_core::plant::{Branching, CountRange};
use tesserae_core::template::{Cell, Template};
use tesserae_core::terrain::LandformParams;
use tesserae_core::testgen::fixtures;
use tesserae_core::{
    AIR, Biome, BiomeId, BiomeName, BiomeSet, BiomeStructure, BiomeTable, CHUNK_SIZE,
    ClimateParams, Clump, GroundParams, MAX_SURFACE_STAGES, MaterialId, Ore, OreTable,
    SurfaceFeature, SurfaceFeatures, SurfacePlant, SurfacePlantKind, SurfaceTree, Voxel, Worldgen,
    WorldgenParams,
};

pub const SEED: u64 = 0x7e55_e2ae_0000_0003;

pub const TRUNK: MaterialId = 210;
const CANOPY: MaterialId = 211;
const GRASS: [MaterialId; 2] = [220, 221];
pub const SAND: MaterialId = 230;
const SANDSTONE: MaterialId = 231;
const FLOOR: MaterialId = 180;
const WALL: MaterialId = 181;
const ROOF: MaterialId = 183;
const FOUNDATION: MaterialId = 185;

pub const FOREST: BiomeId = BiomeId::new(0);
pub const DRY: BiomeId = BiomeId::new(1);

/// Forked and leaning like Terra's broadleaf, in groves.
fn tree() -> SurfaceFeature {
    SurfaceFeature {
        material: TRUNK,
        spacing_m: 7.0,
        plant: Some(SurfacePlant {
            shape: 0,
            kind: SurfacePlantKind::Tree(SurfaceTree {
                branching: Some(Branching {
                    limbs: CountRange { min: 3, max: 4 },
                    start: 0.3,
                    reach_m: 3.0,
                    rise: 0.2,
                    fork: Some(0.3),
                    lean_m: 1.0,
                    tiers: false,
                    lobe_radius_m: 1.2,
                }),
                trunk: TRUNK,
                canopy: CANOPY,
                trunk_height_m: (4.0, 7.0),
                canopy_radius_m: 2.5,
                vines: None,
                fruit: None,
                crowding_m: None,
            }),
        }),
        clump: Some(Clump {
            scale_m: 48.0,
            strength: 0.6,
        }),
        field: None,
    }
}

fn grass(spacing_m: f32) -> SurfaceFeature {
    let mut materials = [AIR; MAX_SURFACE_STAGES];
    materials[..2].copy_from_slice(&GRASS);
    SurfaceFeature {
        material: GRASS[0],
        spacing_m,
        plant: Some(SurfacePlant {
            shape: 5,
            kind: SurfacePlantKind::Stages {
                materials,
                len: 2,
                stage_shapes: [None; MAX_SURFACE_STAGES],
            },
        }),
        clump: None,
        field: None,
    }
}

fn hut() -> Template {
    let [w, h, d] = [7, 5, 6];
    let layers = (0..h)
        .map(|y| {
            (0..d)
                .map(|z| {
                    (0..w)
                        .map(|x| {
                            let edge = x == 0 || z == 0 || x == w - 1 || z == d - 1;
                            match y {
                                0 => Cell::Voxel(Voxel::cube(FLOOR)),
                                _ if y == h - 1 => Cell::Voxel(Voxel::cube(ROOF)),
                                _ if edge => Cell::Voxel(Voxel::cube(WALL)),
                                _ => Cell::Air,
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new("t:hut".into(), 1.0, layers, 0, Some(FOUNDATION), 2.0).unwrap()
}

pub fn templates() -> Vec<Template> {
    vec![hut()]
}

fn biome(name: &str, temperature: f32, humidity: f32) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE,
    }
}

/// A forest with clumped trees over grass, a dry Biome with its own ground, wider caves and huts,
/// and a plain between; ores everywhere and one only under the dry ground; water and caves.
pub fn params() -> WorldgenParams {
    let base = tesserae_core::testgen::world_params().with_default_lake(fixtures().water);
    let world = base.ground.unwrap();
    let hut_row = BiomeStructure {
        template: 0,
        spacing_m: 128.0,
        chance: 0.8,
    };
    let biomes = [
        Biome {
            surface_features: SurfaceFeatures::new(&[tree(), grass(2.0)]).unwrap(),
            ..biome("t:forest", 0.5, 0.75)
        },
        Biome {
            ground: Some(GroundParams {
                rock: SANDSTONE,
                soil: SAND,
                surface: SAND,
            }),
            cave_scale: Some(1.4),
            surface_features: SurfaceFeatures::NONE.with_structures(&[hut_row]).unwrap(),
            ..biome("t:dry", 0.8, 0.2)
        },
        Biome {
            ground: Some(world),
            surface_features: SurfaceFeatures::new(&[grass(4.0)]).unwrap(),
            ..biome("t:plain", 0.4, 0.45)
        },
    ];
    let ore = |material, min_depth_m, biomes| Ore {
        material,
        min_depth_m,
        max_depth_m: 300.0,
        vein_size_m: 6.0,
        rarity: 0.6,
        biomes,
    };
    WorldgenParams {
        generator: 3,
        sea_level_m: 24.0,
        relief_m: 24.0,
        landforms: LandformParams {
            depth_m: 100.0,
            height_m: 200.0,
            overhang_m: 10.0,
            ..LandformParams::default()
        },
        climate: ClimateParams {
            temperature_size_m: 384.0,
            humidity_size_m: 384.0,
        },
        biomes: BiomeTable::new(&biomes).unwrap(),
        ores: OreTable::new(&[
            ore(fixtures().lantern, 4.0, BiomeSet::default()),
            ore(fixtures().glass, 8.0, BiomeSet::new(&[DRY]).unwrap()),
        ])
        .unwrap(),
        ..base
    }
}

pub fn generator(voxel_size: f32) -> Worldgen {
    Worldgen::new(SEED, voxel_size, params()).with_templates(&templates())
}

/// What the Server's level-`level` source draws, as `LodGenerators` makes it: no features and, past
/// level 1, no smoothing, at a doubled Voxel per level.
pub fn coarse(voxel_size: f32, level: u32) -> Worldgen {
    let bare = params().without_surface_features();
    let params = if level > 1 {
        WorldgenParams {
            smoothing: None,
            ..bare
        }
    } else {
        bare
    };
    Worldgen::new(SEED, voxel_size * (1u32 << (level - 1)) as f32, params)
}

/// The metres of a Chunk column's middle.
pub fn middle_m(g: &Worldgen, [cx, cz]: [i32; 2]) -> [f32; 2] {
    let cell_m = CHUNK_SIZE as f32 * g.voxel_size();
    [cx, cz].map(|c| (c as f32 + 0.5) * cell_m)
}

/// The Chunk layer holding a height.
pub fn layer(g: &Worldgen, y_m: f32) -> i32 {
    (y_m / (CHUNK_SIZE as f32 * g.voxel_size())).floor() as i32
}

/// The first Chunk column along a fixed outward square spiral whose middle `suits`.
pub fn scan(g: &Worldgen, mut suits: impl FnMut([f32; 2]) -> bool) -> [i32; 2] {
    const RINGS: i32 = 96;
    for r in 0..RINGS {
        for k in 0..(8 * r).max(1) {
            let side = if r == 0 { 0 } else { k / (2 * r) };
            let t = if r == 0 { 0 } else { k % (2 * r) - r };
            let at = match side {
                0 => [t, -r],
                1 => [r, t],
                2 => [-t, r],
                _ => [-r, -t],
            };
            if suits(middle_m(g, at)) {
                return at;
            }
        }
    }
    panic!("no Chunk column within {RINGS} of the origin suits");
}

fn dry_land(g: &Worldgen, [x, z]: [f32; 2], above_m: f32) -> bool {
    g.water_surface_m(x, z).is_none() && g.surface_height_m(x, z) > g.params().sea_level_m + above_m
}

/// Each case the bench times: a name and the Chunks it generates, in order, from a cold generator.
pub struct Case {
    pub name: &'static str,
    pub voxel_size: f32,
    /// The Server's level-`level` source, or level 0's own generator at 0.
    pub level: u32,
    pub chunks: Vec<[i32; 3]>,
}

impl Case {
    pub fn generator(&self) -> Worldgen {
        match self.level {
            0 => generator(self.voxel_size),
            level => coarse(self.voxel_size, level),
        }
    }
}

/// Every Chunk of the band over a column, bottom to top, that is not an early-out.
pub fn band(g: &Worldgen, [cx, cz]: [i32; 2]) -> Vec<[i32; 3]> {
    let cell_m = CHUNK_SIZE as f32 * g.voxel_size();
    let [x, z] = middle_m(g, [cx, cz]);
    let ground_m = g.surface_height_m(x, z);
    let (lo, hi) = (
        layer(g, ground_m - 2.0 * cell_m),
        layer(g, ground_m + cell_m),
    );
    (lo..=hi).map(|cy| [cx, cy, cz]).collect()
}

pub fn cases(voxel_size: f32) -> Vec<Case> {
    let g = generator(voxel_size);
    let (lowest, highest) = g.surface_bounds_m();
    let straddle = |at: [i32; 2]| {
        let [x, z] = middle_m(&g, at);
        [at[0], layer(&g, g.surface_height_m(x, z)), at[1]]
    };
    let forest = scan(&g, |at| {
        g.biome_at(at[0], at[1]) == Some(FOREST) && dry_land(&g, at, 4.0)
    });
    let dry = scan(&g, |at| {
        g.biome_at(at[0], at[1]) == Some(DRY) && dry_land(&g, at, 4.0)
    });
    let river = scan(&g, |[x, z]| {
        g.water_surface_m(x, z)
            .is_some_and(|w| w > g.params().sea_level_m + 2.0)
    });
    // A cavern: the first Chunk two layers under dry ground with a tenth of its Voxels open.
    let cavern = {
        let cell_m = CHUNK_SIZE as f32 * voxel_size;
        let under = |[x, z]: [f32; 2]| {
            let at = [x, z].map(|m| (m / cell_m).floor() as i32);
            [at[0], layer(&g, g.surface_height_m(x, z)) - 2, at[1]]
        };
        let at = scan(&g, |m| {
            if !dry_land(&g, m, 8.0) {
                return false;
            }
            let [cx, cy, cz] = under(m);
            let chunk = g.chunk(cx, cy, cz);
            let open = (0..CHUNK_SIZE.pow(3))
                .filter(|i| {
                    let (x, y, z) = (
                        i % CHUNK_SIZE,
                        i / CHUNK_SIZE % CHUNK_SIZE,
                        i / CHUNK_SIZE.pow(2),
                    );
                    chunk.get(x, y, z).is_air()
                })
                .count();
            open * 10 >= CHUNK_SIZE.pow(3)
        });
        under(middle_m(&g, at))
    };
    let cell_m = CHUNK_SIZE as f32 * voxel_size;
    let sky = [forest[0], (highest / cell_m).ceil() as i32 + 1, forest[1]];
    let deep = [
        forest[0],
        ((lowest - 64.0) / cell_m).floor() as i32 - 1,
        forest[1],
    ];
    // Level 3 is 4 Voxels to level 0's one: the column over the forest, as the coarse ring asks it.
    let coarse_level = 3;
    let c = coarse(voxel_size, coarse_level);
    let coarse_at = scan(&c, |at| dry_land(&c, at, 4.0));
    let patch: Vec<[i32; 3]> = (-1..=1)
        .flat_map(|dz| (-1..=1).map(move |dx| [forest[0] + dx, forest[1] + dz]))
        .flat_map(|at| band(&g, at))
        .collect();
    vec![
        Case {
            name: "straddling forest",
            voxel_size,
            level: 0,
            chunks: vec![straddle(forest)],
        },
        Case {
            name: "straddling dry",
            voxel_size,
            level: 0,
            chunks: vec![straddle(dry)],
        },
        Case {
            name: "sky",
            voxel_size,
            level: 0,
            chunks: vec![sky],
        },
        Case {
            name: "deep rock",
            voxel_size,
            level: 0,
            chunks: vec![deep],
        },
        Case {
            name: "river",
            voxel_size,
            level: 0,
            chunks: vec![straddle(river)],
        },
        Case {
            name: "cavern",
            voxel_size,
            level: 0,
            chunks: vec![cavern],
        },
        Case {
            name: "coarse level 3",
            voxel_size,
            level: coarse_level,
            chunks: band(&c, coarse_at),
        },
        Case {
            name: "forest 3x3 columns",
            voxel_size,
            level: 0,
            chunks: patch,
        },
    ]
}
