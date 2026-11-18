//! Version 3: a Biome beds layers of rock by depth below its surface.

use std::collections::HashMap;

use tesserae_core::{
    Biome, BiomeName, BiomeTable, CHUNK_SIZE, Chunk, MaterialId, Strata, Stratum, SurfaceFeatures,
    Worldgen, WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0011;
const CLAY: MaterialId = 200;
const BASALT: MaterialId = 201;
/// How far a stratum's edges may undulate, and a Voxel of rounding.
const SLACK_M: f32 = 3.0 + 1.0;

fn params(generator: u32, strata: &[Stratum]) -> WorldgenParams {
    let plain = Biome {
        name: BiomeName::new("t:plain").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE,
    };
    WorldgenParams {
        generator,
        caves: None,
        biomes: BiomeTable::new(&[plain])
            .unwrap()
            .with_strata(&[Strata::new(strata).unwrap()])
            .unwrap(),
        water: None,
        ..tesserae_core::testgen::world_params_v3()
    }
}

fn bedded() -> [Stratum; 2] {
    [
        Stratum {
            material: CLAY,
            depth_m: [4.0, 7.0],
        },
        Stratum {
            material: BASALT,
            depth_m: [20.0, 40.0],
        },
    ]
}

/// Each column's Voxels from its surface down `depth_m`, top first, read from whichever Chunk
/// holds them.
struct Columns {
    generator: Worldgen,
    chunks: HashMap<[i32; 3], Chunk>,
}

impl Columns {
    fn new(generator: Worldgen) -> Self {
        Columns {
            generator,
            chunks: HashMap::new(),
        }
    }

    fn below(&mut self, x_m: f32, z_m: f32, depth_m: f32) -> Column {
        let size = self.generator.voxel_size();
        let surface_m = self.generator.surface_height_m(x_m, z_m);
        let [gx, gz] = [x_m, z_m].map(|m| (m / size).floor() as i32);
        let top = (surface_m / size).floor() as i32;
        let n = CHUNK_SIZE as i32;
        (0..(depth_m / size) as i32)
            .map(|k| {
                let gy = top - k;
                let key = [gx, gy, gz].map(|g| g.div_euclid(n));
                let chunk = self
                    .chunks
                    .entry(key)
                    .or_insert_with(|| self.generator.chunk(key[0], key[1], key[2]));
                let [x, y, z] = [gx, gy, gz].map(|g| g.rem_euclid(n) as usize);
                (surface_m - gy as f32 * size, chunk.get(x, y, z).material())
            })
            .collect()
    }
}

/// A column's Voxels top first, each with how far under the surface it stands.
type Column = Vec<(f32, MaterialId)>;

/// Where in a column each Material starts and how much of it there is, in metres.
fn layer(column: &[(f32, MaterialId)], material: MaterialId, size: f32) -> Option<(f32, f32)> {
    let mut held = column.iter().filter(|(_, m)| *m == material);
    let first = held.next()?.0;
    Some((first, (1 + held.count()) as f32 * size))
}

/// Columns of dry ground, solid all the way down, where nothing else breaks the layers.
fn solid_columns(columns: &mut Columns, depth_m: f32) -> Vec<(f32, f32, Column)> {
    let water = columns.generator.water().map(|w| w.material);
    (0..400)
        .map(|i| ((i % 20) as f32 * 37.0 + 3.0, (i / 20) as f32 * 41.0 + 5.0))
        .filter_map(|(x, z)| {
            let column = columns.below(x, z, depth_m);
            column
                .iter()
                .all(|&(_, m)| m != 0 && Some(m) != water)
                .then_some((x, z, column))
        })
        .collect()
}

#[test]
fn strata_lie_at_their_depth_and_thickness_the_same_at_every_voxel_size() {
    let mut tops = Vec::new();
    for size in [1.0, 0.5] {
        let mut columns = Columns::new(Worldgen::new(SEED, size, params(3, &bedded())));
        let solid = solid_columns(&mut columns, 48.0);
        assert!(
            solid.len() >= 50,
            "{size} m: only {} solid columns",
            solid.len()
        );
        let mut at = Vec::new();
        for (x, z, column) in &solid {
            for (stratum, material) in bedded().iter().map(|s| (s, s.material)) {
                let [top, bottom] = stratum.depth_m;
                let (first_m, thick_m) = layer(column, material, size)
                    .unwrap_or_else(|| panic!("{size} m: ({x}, {z}) has no {material}"));
                assert!(
                    (top..=top + SLACK_M).contains(&first_m),
                    "{size} m: ({x}, {z}): {material} starts {first_m} m down"
                );
                assert!(
                    (thick_m - (bottom - top)).abs() <= 2.0 * size,
                    "{size} m: ({x}, {z}): {material} is {thick_m} m thick"
                );
            }
            at.push(layer(column, CLAY, size).unwrap().0);
        }
        tops.push(at);
    }
    for (a, b) in tops[0].iter().zip(&tops[1]) {
        assert!((a - b).abs() <= 1.0, "the clay starts {a} m and {b} m down");
    }
}

#[test]
fn strata_undulate_rather_than_rule_a_line() {
    let mut columns = Columns::new(Worldgen::new(SEED, 0.5, params(3, &bedded())));
    let starts: std::collections::HashSet<i32> = solid_columns(&mut columns, 12.0)
        .iter()
        .filter_map(|(_, _, c)| layer(c, CLAY, 0.5))
        .map(|(first_m, _)| (first_m * 2.0).round() as i32)
        .collect();
    assert!(starts.len() >= 4, "the clay starts at only {starts:?}");
}

#[test]
fn version_2_lays_no_strata() {
    let mut columns = Columns::new(Worldgen::new(SEED, 1.0, params(2, &bedded())));
    let solid = solid_columns(&mut columns, 48.0);
    assert!(solid.len() >= 50, "only {} solid columns", solid.len());
    for (x, z, column) in solid {
        assert!(
            column.iter().all(|&(_, m)| m != CLAY && m != BASALT),
            "({x}, {z})"
        );
    }
}

#[test]
fn a_stratum_must_run_down_within_96_m_and_a_biome_holds_at_most_8() {
    for depth_m in [
        [-1.0, 4.0],
        [5.0, 5.0],
        [6.0, 2.0],
        [10.0, 97.0],
        [0.0, f32::NAN],
    ] {
        let err = Strata::new(&[Stratum {
            material: CLAY,
            depth_m,
        }])
        .unwrap_err();
        assert_eq!(err.field, "biomes.strata.depth_m", "{depth_m:?}");
    }
    let nine = [Stratum {
        material: CLAY,
        depth_m: [1.0, 2.0],
    }; 9];
    assert_eq!(Strata::new(&nine).unwrap_err().field, "biomes.strata");
}
