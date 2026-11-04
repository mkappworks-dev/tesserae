//! A Biome's ground, through the generator's chunk function: which Materials a
//! column is made of, and everything a Biome must leave alone.

use tesserae_core::testgen::fixtures;
use tesserae_core::{
    Biome, BiomeName, BiomeTable, CHUNK_SIZE, ClimateParams, GroundParams, Voxel, Worldgen,
    WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;

fn biome(name: &str, temperature: f32, ground: Option<GroundParams>) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity: 0.5,
        ground,
    }
}

/// Solid fixture Materials the World's own ground never uses.
fn desert() -> GroundParams {
    GroundParams {
        rock: fixtures().lantern,
        soil: fixtures().glass,
        surface: fixtures().leaves,
    }
}

fn tundra() -> GroundParams {
    GroundParams {
        rock: fixtures().glass,
        soil: fixtures().leaves,
        surface: fixtures().lantern,
    }
}

/// Regions small enough that a few Chunks cross a border.
fn small_regions() -> ClimateParams {
    ClimateParams {
        temperature_size_m: 48.0,
        humidity_size_m: 48.0,
    }
}

/// Cubes only, so the terrain tests see the ground and not its stairs.
fn params(biomes: &[Biome]) -> WorldgenParams {
    WorldgenParams {
        caves: None,
        smoothing: None,
        water: None,
        climate: small_regions(),
        biomes: BiomeTable::new(biomes).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

fn generator(biomes: &[Biome], voxel_size: f32) -> Worldgen {
    Worldgen::new(SEED, voxel_size, params(biomes))
}

/// Every column of the Chunks `-reach..reach` either way of the origin, as the
/// column's metre position and its Voxels from `y = 0` m up past the surface
/// band.
fn columns(g: &Worldgen, reach: i32) -> Vec<((f32, f32), Vec<Voxel>)> {
    let size = CHUNK_SIZE as i32;
    let voxel = g.voxel_size();
    let (_, highest) = g.surface_bounds_m();
    let rows = (highest / (CHUNK_SIZE as f32 * voxel)).ceil() as i32;
    let mut out = Vec::new();
    for cz in -reach..reach {
        for cx in -reach..reach {
            let chunks: Vec<_> = (0..=rows).map(|cy| g.chunk(cx, cy, cz)).collect();
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let at = (
                        (cx * size + x as i32) as f32 * voxel,
                        (cz * size + z as i32) as f32 * voxel,
                    );
                    let voxels = chunks
                        .iter()
                        .flat_map(|c| (0..CHUNK_SIZE).map(move |y| c.get(x, y, z)))
                        .collect();
                    out.push((at, voxels));
                }
            }
        }
    }
    out
}

fn top(voxels: &[Voxel]) -> usize {
    voxels.iter().rposition(|v| !v.is_air()).expect("a surface")
}

#[test]
fn a_column_is_made_of_the_ground_of_its_biome() {
    let g = generator(
        &[
            biome("cold", 0.2, Some(tundra())),
            biome("warm", 0.8, Some(desert())),
        ],
        1.0,
    );
    let mut seen = [0usize; 2];
    for ((x, z), voxels) in columns(&g, 4) {
        let id = g.biome_at(x, z).expect("a Biome");
        let ground = [tundra(), desert()][id.index()];
        let top = top(&voxels);
        assert_eq!(voxels[top].material(), ground.surface, "surface at {x},{z}");
        assert_eq!(voxels[top - 1].material(), ground.soil, "soil at {x},{z}");
        assert_eq!(voxels[20].material(), ground.rock, "rock at {x},{z}");
        seen[id.index()] += 1;
    }
    // Without both, the loop above checked one Biome.
    assert!(seen.iter().all(|&n| n > 2_000), "{seen:?}");
}

#[test]
fn a_biome_without_ground_has_the_worlds() {
    let world = fixtures().ground;
    let g = generator(
        &[biome("cold", 0.2, None), biome("warm", 0.8, Some(desert()))],
        1.0,
    );
    let mut seen = [0usize; 2];
    for ((x, z), voxels) in columns(&g, 4) {
        let id = g.biome_at(x, z).expect("a Biome");
        let ground = [world, desert()][id.index()];
        let top = top(&voxels);
        assert_eq!(voxels[top].material(), ground.surface, "surface at {x},{z}");
        assert_eq!(voxels[top - 1].material(), ground.soil, "soil at {x},{z}");
        assert_eq!(voxels[20].material(), ground.rock, "rock at {x},{z}");
        seen[id.index()] += 1;
    }
    assert!(seen.iter().all(|&n| n > 2_000), "{seen:?}");
}

/// The shape of every column with the Materials taken out: which Voxels are
/// air, and the Shape and Orientation of the rest.
fn silhouette(g: &Worldgen, reach: i32) -> Vec<Vec<(bool, u8, u8)>> {
    columns(g, reach)
        .into_iter()
        .map(|(_, voxels)| {
            voxels
                .iter()
                .map(|v| (v.is_air(), v.shape(), v.orientation()))
                .collect()
        })
        .collect()
}

#[test]
fn biomes_change_no_height_cave_lake_or_stair() {
    let defaults = tesserae_core::testgen::world_params();
    let plain = WorldgenParams {
        climate: small_regions(),
        smoothing: Some(tesserae_core::SmoothingParams { min_rise_m: 0.0 }),
        water: Some(tesserae_core::worldgen::WaterParams {
            level_m: 34.0,
            material: fixtures().water,
        }),
        ..defaults
    };
    let with = WorldgenParams {
        biomes: BiomeTable::new(&[
            biome("cold", 0.2, Some(tundra())),
            biome("warm", 0.8, Some(desert())),
        ])
        .unwrap(),
        ..plain
    };
    for voxel_size in [1.0, 0.5] {
        let (a, b) = (
            Worldgen::new(SEED, voxel_size, plain),
            Worldgen::new(SEED, voxel_size, with),
        );
        assert_eq!(a.surface_bounds_m(), b.surface_bounds_m());
        assert_eq!(a.water(), b.water());
        for (x, z) in [(0.0, 0.0), (-130.0, 77.0), (41.5, -203.25)] {
            assert_eq!(a.surface_height_m(x, z), b.surface_height_m(x, z));
        }
        let reach = (96.0 / (CHUNK_SIZE as f32 * voxel_size)).ceil() as i32;
        let shaped = silhouette(&a, reach);
        assert!(
            shaped
                .iter()
                .flatten()
                .any(|&(air, shape, _)| !air && shape != 0),
            "no stairs at {voxel_size} m"
        );
        assert_eq!(shaped, silhouette(&b, reach), "at {voxel_size} m");
    }
}

#[test]
fn a_stair_takes_the_surface_of_its_columns_biome() {
    let g = Worldgen::new(
        SEED,
        1.0,
        WorldgenParams {
            smoothing: Some(tesserae_core::SmoothingParams { min_rise_m: 0.0 }),
            ..params(&[
                biome("cold", 0.2, Some(tundra())),
                biome("warm", 0.8, Some(desert())),
            ])
        },
    );
    let mut stairs = 0;
    for ((x, z), voxels) in columns(&g, 4) {
        let ground = [tundra(), desert()][g.biome_at(x, z).unwrap().index()];
        for v in voxels.iter().filter(|v| !v.is_air() && v.shape() != 0) {
            assert_eq!(v.material(), ground.surface, "at {x},{z}");
            stairs += 1;
        }
    }
    assert!(stairs > 100, "{stairs} stairs");
}

#[test]
fn a_biome_makes_the_same_ground_at_the_same_metres_at_any_voxel_size() {
    let biomes = [
        biome("cold", 0.2, Some(tundra())),
        biome("warm", 0.8, Some(desert())),
    ];
    for voxel_size in [1.0, 0.5, 0.25] {
        let g = generator(&biomes, voxel_size);
        let reach = (128.0 / (CHUNK_SIZE as f32 * voxel_size)).ceil() as i32;
        let mut seen = [0usize; 2];
        for ((x, z), voxels) in columns(&g, reach).into_iter().step_by(7) {
            let id = g.biome_at(x, z).unwrap();
            let want = [tundra(), desert()][id.index()];
            assert_eq!(voxels[top(&voxels)].material(), want.surface, "{x},{z}");
            seen[id.index()] += 1;
        }
        assert!(seen.iter().all(|&n| n > 100), "{seen:?} at {voxel_size} m");
    }
}

#[test]
fn generator_version_1_ignores_biome_ground() {
    let biomes = [
        biome("cold", 0.2, Some(tundra())),
        biome("warm", 0.8, Some(desert())),
    ];
    let v1 = WorldgenParams {
        generator: 1,
        ..params(&biomes)
    };
    let plain = WorldgenParams {
        biomes: BiomeTable::default(),
        ..v1
    };
    let (a, b) = (
        Worldgen::new(SEED, 1.0, v1),
        Worldgen::new(SEED, 1.0, plain),
    );
    for ((_, x), (_, y)) in columns(&a, 3).into_iter().zip(columns(&b, 3)) {
        assert_eq!(x, y);
    }
}

#[test]
fn an_empty_table_or_biomes_without_ground_generate_the_worlds_own_ground() {
    let none = generator(&[], 1.0);
    let bare = generator(&[biome("cold", 0.2, None), biome("warm", 0.8, None)], 1.0);
    let same_as_world = generator(&[biome("only", 0.5, Some(fixtures().ground))], 1.0);
    let want = columns(&none, 3);
    assert_eq!(columns(&bare, 3), want);
    assert_eq!(columns(&same_as_world, 3), want);
}

#[test]
fn deep_rock_is_each_columns_own_where_biomes_disagree_about_rock() {
    let g = generator(
        &[
            biome("cold", 0.2, Some(tundra())),
            biome("warm", 0.8, Some(desert())),
        ],
        1.0,
    );
    let (mut mixed, mut uniform) = (0, 0);
    for cz in -4..4 {
        for cx in -4..4 {
            let chunk = g.chunk(cx, -2, cz);
            uniform += chunk.uniform().is_some() as usize;
            mixed += chunk.uniform().is_none() as usize;
            for (x, z) in [(0, 0), (31, 0), (0, 31), (31, 31), (13, 7)] {
                let (x_m, z_m) = ((cx * 32 + x) as f32, (cz * 32 + z) as f32);
                let want = [tundra(), desert()][g.biome_at(x_m, z_m).unwrap().index()].rock;
                for y in [0, 15, 31] {
                    let got = chunk.get(x as usize, y, z as usize);
                    assert_eq!(got, Voxel::cube(want), "{cx},{cz} column {x},{z}");
                }
            }
        }
    }
    // Chunks wholly inside one Biome stay uniform for the fast paths, and
    // those across a border are not.
    assert!(mixed > 0 && uniform > 0, "mixed {mixed}, uniform {uniform}");
}

#[test]
fn deep_rock_stays_one_uniform_chunk_when_no_biome_changes_rock() {
    let same_rock = GroundParams {
        rock: fixtures().ground.rock,
        ..desert()
    };
    let g = generator(
        &[
            biome("cold", 0.2, None),
            biome("warm", 0.8, Some(same_rock)),
        ],
        1.0,
    );
    for cz in -4..4 {
        for cx in -4..4 {
            assert_eq!(
                g.chunk(cx, -2, cz).uniform(),
                Some(Voxel::cube(fixtures().ground.rock)),
                "{cx},{cz}"
            );
        }
    }
}

#[test]
fn a_lone_biome_makes_deep_rock_one_uniform_chunk_of_its_rock() {
    let g = generator(&[biome("only", 0.5, Some(desert()))], 1.0);
    assert_eq!(
        g.chunk(3, -2, -5).uniform(),
        Some(Voxel::cube(desert().rock))
    );
}

#[test]
fn caves_carve_deep_biome_rock_without_changing_which_voxels_are_air() {
    let with_caves = |biomes: &[Biome]| {
        Worldgen::new(
            SEED,
            1.0,
            WorldgenParams {
                caves: Some(tesserae_core::CaveParams {
                    floor_m: -200.0,
                    radius_m: 6.0,
                    ..Default::default()
                }),
                ..params(biomes)
            },
        )
    };
    let plain = with_caves(&[]);
    let varied = with_caves(&[
        biome("cold", 0.2, Some(tundra())),
        biome("warm", 0.8, Some(desert())),
    ]);
    let mut carved = 0;
    for cz in -3..3 {
        for cx in -3..3 {
            let (a, b) = (plain.chunk(cx, -2, cz), varied.chunk(cx, -2, cz));
            for i in 0..CHUNK_SIZE.pow(3) {
                let at = (
                    i % CHUNK_SIZE,
                    i / CHUNK_SIZE % CHUNK_SIZE,
                    i / CHUNK_SIZE.pow(2),
                );
                let (p, v) = (a.get(at.0, at.1, at.2), b.get(at.0, at.1, at.2));
                assert_eq!(p.is_air(), v.is_air(), "{cx},{cz} {at:?}");
                carved += p.is_air() as usize;
            }
        }
    }
    assert!(carved > 0, "no cave in the Chunks checked");
}

#[test]
fn a_biomes_ground_is_held_to_the_rules_of_the_worlds() {
    let tables = &fixtures().tables;
    let checked =
        |ground: GroundParams| params(&[biome("cold", 0.2, Some(ground))]).validate_ground(tables);
    assert!(checked(desert()).is_ok());
    for (what, bad) in [
        (
            "a fluid",
            GroundParams {
                soil: fixtures().water,
                ..desert()
            },
        ),
        (
            "unregistered",
            GroundParams {
                rock: 9_999,
                ..desert()
            },
        ),
        (
            "air",
            GroundParams {
                surface: 0,
                ..desert()
            },
        ),
    ] {
        assert!(checked(bad).is_err(), "{what}");
    }
}
