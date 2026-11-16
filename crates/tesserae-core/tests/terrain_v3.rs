//! Generator version 3: terrain lives in a declared vertical band around Sea level.

use tesserae_core::terrain::LandformParams;
use tesserae_core::{CHUNK_SIZE, Worldgen, WorldgenParams};

const SEED: u64 = 0x7e55_e2ae_0000_0001;

fn landforms() -> LandformParams {
    LandformParams {
        depth_m: 100.0,
        height_m: 200.0,
        overhang_m: 10.0,
        ..LandformParams::default()
    }
}

fn v3() -> WorldgenParams {
    WorldgenParams {
        generator: 3,
        sea_level_m: 24.0,
        relief_m: 24.0,
        landforms: landforms(),
        ..tesserae_core::testgen::world_params()
    }
}

#[test]
fn version_3_validates() {
    assert_eq!(v3().validate(), Ok(()));
}

#[test]
fn a_band_that_does_not_hold_the_relief_is_refused() {
    let bad = WorldgenParams {
        landforms: LandformParams {
            height_m: 0.0,
            ..landforms()
        },
        ..v3()
    };
    assert!(bad.validate().is_err());
}

#[test]
fn version_3_bounds_the_surface_by_the_band_and_the_overhang() {
    let (lowest, highest) = Worldgen::new(SEED, 1.0, v3()).surface_bounds_m();
    assert_eq!(lowest, 24.0 - 100.0 - 10.0);
    assert_eq!(highest, 24.0 + 200.0 + 10.0);
}

#[test]
fn version_2_keeps_its_old_bound() {
    let v2 = WorldgenParams {
        generator: 2,
        ..v3()
    };
    assert_eq!(
        Worldgen::new(SEED, 1.0, v2).surface_bounds_m(),
        (24.0, 48.0)
    );
}

#[test]
fn no_voxel_is_solid_above_the_version_3_bound() {
    let g = Worldgen::new(SEED, 1.0, v3());
    let (_, highest) = g.surface_bounds_m();
    let cy = (highest / CHUNK_SIZE as f32).ceil() as i32;
    for cz in -2..2 {
        for cx in -2..2 {
            assert!(g.chunk(cx, cy, cz).uniform().is_some_and(|v| v.is_air()));
        }
    }
}

fn grid(g: &Worldgen, step_m: f32, half: i32) -> Vec<f32> {
    (-half..=half)
        .flat_map(|j| (-half..=half).map(move |i| (i, j)))
        .map(|(i, j)| g.surface_height_m(i as f32 * step_m, j as f32 * step_m))
        .collect()
}

#[test]
fn every_height_lies_in_the_band() {
    let g = Worldgen::new(SEED, 1.0, v3());
    let (lo, hi) = (24.0 - 100.0, 24.0 + 200.0);
    for h in grid(&g, 211.0, 40) {
        assert!((lo..=hi).contains(&h), "{h}");
    }
}

#[test]
fn a_world_has_ocean_plains_and_high_mountains() {
    let g = Worldgen::new(SEED, 1.0, v3());
    let heights = grid(&g, 350.0, 40);
    let n = heights.len() as f32;
    let ocean = heights.iter().filter(|&&h| h < 24.0).count() as f32 / n;
    assert!((0.15..=0.65).contains(&ocean), "ocean share {ocean}");
    let plains = heights
        .iter()
        .filter(|&&h| (24.0..24.0 + 40.0).contains(&h))
        .count() as f32
        / n;
    assert!(plains > 0.1, "plains share {plains}");
    let peak = heights.iter().cloned().fold(f32::MIN, f32::max);
    assert!(peak > 24.0 + 100.0, "highest ground {peak}");
    let deepest = heights.iter().cloned().fold(f32::MAX, f32::min);
    assert!(deepest < 24.0 - 40.0, "deepest floor {deepest}");
}

#[test]
fn the_ground_is_the_same_in_metres_at_every_voxel_size() {
    let a = Worldgen::new(SEED, 1.0, v3());
    let b = Worldgen::new(SEED, 0.25, v3());
    for (x, z) in [(0.0, 0.0), (513.5, -820.25), (-4000.0, 3333.0)] {
        assert_eq!(a.surface_height_m(x, z), b.surface_height_m(x, z));
    }
}

#[test]
fn another_seed_is_another_world() {
    let a = grid(&Worldgen::new(SEED, 1.0, v3()), 500.0, 10);
    let b = grid(&Worldgen::new(SEED + 1, 1.0, v3()), 500.0, 10);
    assert_ne!(a, b);
}

#[test]
fn chunks_hold_the_surface_to_within_a_voxel() {
    let g = Worldgen::new(SEED, 1.0, v3());
    let (cx, cz) = (3, -2);
    let (x_m, z_m) = (
        (cx * CHUNK_SIZE as i32) as f32 + 5.0,
        (cz * CHUNK_SIZE as i32) as f32 + 9.0,
    );
    let h = g.surface_height_m(x_m, z_m);
    let cy = (h / CHUNK_SIZE as f32).floor() as i32;
    let chunk = g.chunk(cx, cy, cz);
    let local = (h - (cy * CHUNK_SIZE as i32) as f32).floor() as usize;
    assert!(!chunk.get(5, local.min(CHUNK_SIZE - 1), 9).is_air());
}

fn wet_world() -> WorldgenParams {
    WorldgenParams {
        water: Some(tesserae_core::worldgen::WaterParams {
            level_m: 9999.0,
            material: tesserae_core::testgen::fixtures().water,
        }),
        ..v3()
    }
}

/// A column the sea covers by at least `depth_m`, found by walking a fixed grid.
fn ocean_column(g: &Worldgen, depth_m: f32) -> (f32, f32) {
    (-60..60)
        .flat_map(|j| (-60..60).map(move |i| (i as f32 * 100.0, j as f32 * 100.0)))
        .find(|&(x, z)| g.surface_height_m(x, z) < 24.0 - depth_m)
        .expect("a World has ocean")
}

#[test]
fn the_sea_fills_ocean_columns_up_to_sea_level_and_no_higher() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let water = tesserae_core::testgen::fixtures().water;
    let (x, z) = ocean_column(&g, 6.0);
    let (cx, cz) = ((x / 32.0).floor() as i32, (z / 32.0).floor() as i32);
    let (lx, lz) = (
        (x - cx as f32 * 32.0) as usize,
        (z - cz as f32 * 32.0) as usize,
    );
    let at = |y_m: f32| {
        let cy = (y_m / 32.0).floor() as i32;
        g.chunk(cx, cy, cz)
            .get(lx, (y_m - cy as f32 * 32.0) as usize, lz)
    };
    assert_eq!(at(22.0).material(), water, "just under Sea level");
    assert!(at(24.0).is_air(), "at Sea level");
    assert!(at(30.0).is_air(), "above it");
}

#[test]
fn water_surface_is_sea_level_over_ocean_and_none_over_land() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let (mut wet, mut dry) = (0, 0);
    for j in -30..30 {
        for i in -30..30 {
            let (x, z) = (i as f32 * 211.0, j as f32 * 211.0);
            match g.water_surface_m(x, z) {
                Some(level) => {
                    assert_eq!(level, 24.0);
                    assert!(g.surface_height_m(x, z) < 24.0);
                    wet += 1;
                }
                None => {
                    assert!(g.surface_height_m(x, z) >= 24.0);
                    dry += 1;
                }
            }
        }
    }
    assert!(wet > 100 && dry > 100, "{wet} wet, {dry} dry");
}

#[test]
fn without_a_fluid_the_basins_are_dry() {
    let g = Worldgen::new(SEED, 1.0, v3());
    assert_eq!(g.water_surface_m(0.0, 0.0), None);
    let (x, z) = ocean_column(&g, 6.0);
    assert_eq!(g.water_surface_m(x, z), None);
}

#[test]
fn a_coarse_generator_draws_the_same_coast() {
    let fine = Worldgen::new(SEED, 1.0, wet_world());
    let coarse = Worldgen::new(SEED, 4.0, wet_world());
    for j in -30..30 {
        for i in -30..30 {
            let (x, z) = (i as f32 * 137.0, j as f32 * 137.0);
            assert_eq!(fine.water_surface_m(x, z), coarse.water_surface_m(x, z));
        }
    }
}

fn dry_v3() -> WorldgenParams {
    WorldgenParams {
        caves: None,
        smoothing: None,
        ..v3()
    }
}

/// Whether a column of chunks holds a solid Voxel over an air one, scanning a metre column.
fn has_overhang(g: &Worldgen, x: i32, z: i32, from_m: i32, to_m: i32) -> bool {
    let (cx, cz) = (x.div_euclid(32), z.div_euclid(32));
    let (lx, lz) = (x.rem_euclid(32) as usize, z.rem_euclid(32) as usize);
    let mut seen_air = false;
    for y in from_m..to_m {
        let cy = y.div_euclid(32);
        let v = g.chunk(cx, cy, cz).get(lx, y.rem_euclid(32) as usize, lz);
        if v.is_air() {
            seen_air = true;
        } else if seen_air {
            return true;
        }
    }
    false
}

fn columns_where(g: &Worldgen, keep: impl Fn(f32) -> bool, want: usize) -> Vec<(i32, i32)> {
    (-80..80)
        .flat_map(|j| (-80..80).map(move |i| (i * 61, j * 61)))
        .filter(|&(x, z)| keep(g.landform_fields(x as f32, z as f32).unwrap().ruggedness))
        .take(want)
        .collect()
}

#[test]
fn flat_ground_is_a_heightfield() {
    let g = Worldgen::new(SEED, 1.0, dry_v3());
    for (x, z) in columns_where(&g, |r| r < 0.05, 12) {
        let h = g.surface_height_m(x as f32, z as f32);
        assert!(
            !has_overhang(&g, x, z, h as i32 - 20, h as i32 + 20),
            "{x},{z}"
        );
    }
}

#[test]
fn rugged_ground_has_overhangs_and_arches() {
    let g = Worldgen::new(SEED, 1.0, dry_v3());
    let rugged = columns_where(&g, |r| r > 0.5, 60);
    assert!(rugged.len() >= 20, "{} rugged columns", rugged.len());
    let with = rugged
        .iter()
        .filter(|&&(x, z)| {
            let h = g.surface_height_m(x as f32, z as f32);
            has_overhang(&g, x, z, h as i32 - 14, h as i32 + 14)
        })
        .count();
    assert!(with * 5 >= rugged.len(), "{with} of {}", rugged.len());
}

#[test]
fn turning_overhangs_off_leaves_a_heightfield_everywhere() {
    let params = WorldgenParams {
        landforms: LandformParams {
            overhangs: false,
            ..landforms()
        },
        ..dry_v3()
    };
    let g = Worldgen::new(SEED, 1.0, params);
    for (x, z) in columns_where(&g, |r| r > 0.5, 20) {
        let h = g.surface_height_m(x as f32, z as f32);
        assert!(
            !has_overhang(&g, x, z, h as i32 - 14, h as i32 + 14),
            "{x},{z}"
        );
    }
}

#[test]
fn nothing_stands_more_than_the_overhang_above_the_base_surface() {
    let g = Worldgen::new(SEED, 1.0, dry_v3());
    for (x, z) in columns_where(&g, |r| r > 0.5, 20) {
        let h = g.surface_height_m(x as f32, z as f32);
        let (cx, cz) = (x.div_euclid(32), z.div_euclid(32));
        let top = ((h + 10.0 + 6.0) as i32).max(0);
        for y in (h as i32 + 12)..top + 20 {
            let v = g.chunk(cx, y.div_euclid(32), cz).get(
                x.rem_euclid(32) as usize,
                y.rem_euclid(32) as usize,
                z.rem_euclid(32) as usize,
            );
            assert!(v.is_air(), "solid at {x},{y},{z} over ground {h}");
        }
    }
}
