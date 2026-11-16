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
