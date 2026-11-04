//! A Biome's cave tuning, through the generator's chunk function: how much rock
//! its columns lose to tunnels, and everything tuning must leave alone.

use tesserae_core::{
    Biome, BiomeName, BiomeTable, CHUNK_SIZE, Chunk, ClimateParams, Worldgen, WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0006;

fn biome(name: &str, temperature: f32, cave_scale: Option<f32>) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale,
    }
}

/// Regions small enough that a few Chunks cross a border.
fn small_regions() -> ClimateParams {
    ClimateParams {
        temperature_size_m: 96.0,
        humidity_size_m: 96.0,
    }
}

/// Cubes only, so the carving is the only thing that differs between Worlds.
fn params(biomes: &[Biome]) -> WorldgenParams {
    WorldgenParams {
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

/// Every Chunk from the cave floor up to the surface band, `reach` Chunks either
/// way of the origin at the generator's Voxel size.
fn chunks(g: &Worldgen, reach: i32) -> Vec<((i32, i32, i32), Chunk)> {
    let cell_m = CHUNK_SIZE as f32 * g.voxel_size();
    let floor = g.params().caves.expect("caves").floor_m;
    let (_, highest) = g.surface_bounds_m();
    let (lo, hi) = (
        (floor / cell_m).floor() as i32,
        (highest / cell_m).ceil() as i32,
    );
    let mut out = Vec::new();
    for cz in -reach..reach {
        for cx in -reach..reach {
            for cy in lo..=hi {
                out.push(((cx, cy, cz), g.chunk(cx, cy, cz)));
            }
        }
    }
    out
}

/// The Voxels of the rock layer that are air, per column, with the column's
/// metre position: tunnels and nothing else, since the layers above are
/// excluded and nothing else is carved.
fn tunnels(g: &Worldgen, reach: i32) -> Vec<((f32, f32), usize)> {
    let size = CHUNK_SIZE as i32;
    let voxel = g.voxel_size();
    let soil = g.params().soil_depth_m;
    let floor = g.params().caves.expect("caves").floor_m;
    let mut per_column = std::collections::BTreeMap::<(i32, i32), usize>::new();
    for ((cx, cy, cz), chunk) in chunks(g, reach) {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let (x_m, z_m) = (
                    (cx * size + x as i32) as f32 * voxel,
                    (cz * size + z as i32) as f32 * voxel,
                );
                let rock_top = g.surface_height_m(x_m, z_m) - soil - voxel;
                let n = per_column
                    .entry((cx * size + x as i32, cz * size + z as i32))
                    .or_default();
                for y in 0..CHUNK_SIZE {
                    let y_m = (cy * size + y as i32) as f32 * voxel;
                    if y_m >= floor && y_m < rock_top && chunk.get(x, y, z).is_air() {
                        *n += 1;
                    }
                }
            }
        }
    }
    per_column
        .into_iter()
        .map(|((x, z), n)| ((x as f32 * voxel, z as f32 * voxel), n))
        .collect()
}

fn total(g: &Worldgen, reach: i32) -> usize {
    tunnels(g, reach).iter().map(|(_, n)| n).sum()
}

#[test]
fn a_biome_with_caves_off_has_none() {
    let g = generator(&[biome("only", 0.5, Some(0.0))], 1.0);
    assert_eq!(total(&g, 6), 0);
}

#[test]
fn a_biome_with_a_larger_scale_carves_measurably_more() {
    let plain = total(&generator(&[biome("only", 0.5, None)], 1.0), 6);
    let wide = total(&generator(&[biome("only", 0.5, Some(2.0))], 1.0), 6);
    let narrow = total(&generator(&[biome("only", 0.5, Some(0.5))], 1.0), 6);
    assert!(plain > 10_000, "the World has caves to scale: {plain}");
    assert!(
        wide * 2 > plain * 3,
        "a scale of 2 carves {wide} against {plain}"
    );
    assert!(
        narrow * 3 < plain * 2,
        "a scale of 0.5 carves {narrow} against {plain}"
    );
}

#[test]
fn neighbouring_biomes_carve_each_by_their_own_scale() {
    let g = generator(
        &[biome("cold", 0.1, Some(0.0)), biome("warm", 0.9, Some(2.0))],
        1.0,
    );
    let mut per_biome = [(0usize, 0usize); 2];
    for ((x, z), n) in tunnels(&g, 8) {
        let id = g.biome_at(x, z).expect("a Biome").index();
        per_biome[id].0 += 1;
        per_biome[id].1 += n;
    }
    let mean = |(columns, air): (usize, usize)| air as f32 / columns as f32;
    assert!(per_biome.iter().all(|&(columns, _)| columns > 5_000));
    assert!(
        mean(per_biome[0]) * 3.0 < mean(per_biome[1]),
        "{per_biome:?}"
    );
}

#[test]
fn a_cave_scale_outside_zero_to_four_is_refused() {
    for scale in [-0.1, 4.01, f32::NAN, f32::INFINITY] {
        let err = BiomeTable::new(&[biome("odd", 0.5, Some(scale))]).unwrap_err();
        assert_eq!(err.field, "biomes.cave_scale", "{scale}");
    }
    for scale in [0.0, 1.0, 4.0] {
        assert!(BiomeTable::new(&[biome("fine", 0.5, Some(scale))]).is_ok());
    }
}
