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
fn water_surface_is_the_water_over_wet_ground_and_none_over_land() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let (mut wet, mut dry) = (0, 0);
    for j in -30..30 {
        for i in -30..30 {
            let (x, z) = (i as f32 * 211.0, j as f32 * 211.0);
            match g.water_surface_m(x, z) {
                Some(level) => {
                    assert!(level >= 24.0, "sea or higher inland water: {level}");
                    assert!(g.surface_height_m(x, z) < level);
                    wet += 1;
                }
                None => {
                    assert!(g.surface_height_m(x, z) >= g.water_level_m(x, z).unwrap());
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

/// Up to `want` matching columns spread over the whole grid, not the first run of them.
fn columns_where(g: &Worldgen, keep: impl Fn(f32) -> bool, want: usize) -> Vec<(i32, i32)> {
    let all: Vec<_> = (-80..80)
        .flat_map(|j| (-80..80).map(move |i| (i * 61, j * 61)))
        .filter(|&(x, z)| keep(g.landform_fields(x as f32, z as f32).unwrap().ruggedness))
        .collect();
    let every = (all.len() / want.max(1)).max(1);
    all.into_iter().step_by(every).take(want).collect()
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
    let rugged = columns_where(&g, |r| r > 0.7, 60);
    assert!(rugged.len() >= 20, "{} rugged columns", rugged.len());
    let with = rugged
        .iter()
        .filter(|&&(x, z)| {
            let h = g.surface_height_m(x as f32, z as f32);
            has_overhang(&g, x, z, h as i32 - 14, h as i32 + 14)
        })
        .count();
    assert!(with * 10 >= rugged.len(), "{with} of {}", rugged.len());
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

fn wet_columns(g: &Worldgen, x0: f32, z0: f32, n: i32, step: f32) -> Vec<(f32, f32, f32)> {
    (0..n)
        .flat_map(|j| (0..n).map(move |i| (x0 + i as f32 * step, z0 + j as f32 * step)))
        .filter_map(|(x, z)| {
            let level = g.water_level_m(x, z)?;
            (g.surface_height_m(x, z) < level && level > 24.5).then_some((x, z, level))
        })
        .collect()
}

#[test]
fn inland_water_exists_and_sits_above_the_sea() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let inland = wet_columns(&g, -4096.0, -4096.0, 128, 64.0);
    assert!(inland.len() > 40, "{} inland wet columns", inland.len());
}

#[test]
fn drainage_gives_the_same_answers_with_a_cold_cache() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let at = |g: &Worldgen| {
        (0..40)
            .flat_map(|j| (0..40).map(move |i| (i as f32 * 53.0 - 900.0, j as f32 * 71.0 - 1300.0)))
            .map(|(x, z)| (g.surface_height_m(x, z), g.water_level_m(x, z)))
            .collect::<Vec<_>>()
    };
    let warm = at(&g);
    g.forget_regions();
    let cold = at(&g);
    let fresh = at(&Worldgen::new(SEED, 1.0, wet_world()));
    assert_eq!(warm, cold);
    assert_eq!(warm, fresh);
}

#[test]
fn lakes_are_flat_and_hold_water_above_their_floor() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let wet = wet_columns(&g, -4096.0, -4096.0, 256, 32.0);
    assert!(!wet.is_empty());
    for (x, z, level) in wet.iter().take(2000) {
        assert!(g.surface_height_m(*x, *z) < *level);
    }
}

#[test]
fn a_river_valley_is_not_a_cliff_across_a_region_border() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let mut worst = 0.0f32;
    for k in 0..2000 {
        let z = k as f32 * 3.7 - 3000.0;
        for border in [-2048.0f32, 0.0, 2048.0] {
            let step =
                (g.surface_height_m(border + 0.5, z) - g.surface_height_m(border - 0.5, z)).abs();
            worst = worst.max(step);
        }
    }
    assert!(worst < 8.0, "a {worst} m step across a region border");
}

#[test]
fn rivers_and_lakes_are_in_the_same_metres_at_every_voxel_size() {
    let fine = Worldgen::new(SEED, 0.5, wet_world());
    let coarse = Worldgen::new(SEED, 4.0, wet_world());
    for j in 0..40 {
        for i in 0..40 {
            let (x, z) = (i as f32 * 97.0 - 1900.0, j as f32 * 83.0 - 1600.0);
            assert_eq!(fine.water_level_m(x, z), coarse.water_level_m(x, z));
            assert_eq!(fine.surface_height_m(x, z), coarse.surface_height_m(x, z));
        }
    }
}

fn caved_v3() -> WorldgenParams {
    WorldgenParams {
        // Tunnels thinner than a Voxel, so what a column holds is cavern and nothing else.
        caves: Some(tesserae_core::CaveParams {
            radius_m: 0.001,
            ..tesserae_core::CaveParams::default()
        }),
        smoothing: None,
        ..v3()
    }
}

/// Whether each Voxel of the metre column `x, z` from `from_m` up to `to_m` is air.
fn air_column(g: &Worldgen, x: i32, z: i32, from_m: i32, to_m: i32) -> Vec<bool> {
    let (cx, cz) = (x.div_euclid(32), z.div_euclid(32));
    let (lx, lz) = (x.rem_euclid(32) as usize, z.rem_euclid(32) as usize);
    let mut out = Vec::new();
    let mut y = from_m;
    while y < to_m {
        let cy = y.div_euclid(32);
        let chunk = g.chunk(cx, cy, cz);
        while y < to_m && y.div_euclid(32) == cy {
            out.push(chunk.get(lx, y.rem_euclid(32) as usize, lz).is_air());
            y += 1;
        }
    }
    out
}

#[test]
fn caverns_exist_deep_under_mountains_and_never_breach_the_surface() {
    let g = Worldgen::new(SEED, 1.0, caved_v3());
    let (mut hollow, mut breach) = (0, 0);
    for j in -16..16 {
        for i in -16..16 {
            let (x, z) = (i * 233, j * 241);
            let h = g.surface_height_m(x as f32, z as f32) as i32;
            if h < 40 {
                continue;
            }
            let col = air_column(&g, x, z, h - 90, h - 14);
            // Index 0 is 90 m down; the last 6 are 20 to 14 m down, clear of the roof's own margin.
            breach += col[col.len() - 6..].iter().filter(|&&a| a).count();
            hollow += col[..col.len() - 6].iter().filter(|&&a| a).count();
        }
    }
    assert_eq!(breach, 0, "air within 20 m of the ground");
    assert!(hollow > 100, "{hollow} cavern voxels sampled");
}

#[test]
fn rock_below_the_band_floor_stays_uniform() {
    let g = Worldgen::new(SEED, 1.0, caved_v3());
    let below = ((24.0f32 - 100.0 - 10.0 - 3.0) / 32.0).floor() as i32 - 1;
    for (cx, cz) in [(0, 0), (40, -30), (-90, 77)] {
        assert!(g.chunk(cx, below, cz).uniform().is_some());
    }
}

#[test]
fn ravines_cut_narrow_slots_into_rugged_land() {
    let g = Worldgen::new(SEED, 1.0, v3());
    let mut slots = 0;
    for j in -60..60 {
        for i in -60..60 {
            let (x, z) = (i as f32 * 97.0, j as f32 * 89.0);
            if g.landform_fields(x, z).unwrap().ruggedness < 0.3 {
                continue;
            }
            // A slot is a point far lower than the ground ten metres either side of it.
            for k in 0..40 {
                let x = x + k as f32 * 7.0;
                let (a, b, c) = (
                    g.surface_height_m(x - 10.0, z),
                    g.surface_height_m(x, z),
                    g.surface_height_m(x + 10.0, z),
                );
                if a - b > 18.0 && c - b > 18.0 {
                    slots += 1;
                }
            }
        }
    }
    assert!(slots > 0, "no ravines found");
}

#[test]
fn spawning_finds_dry_land_clear_of_cliffs_and_water() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    // Start from the middle of an ocean.
    let (x, z) = ocean_column(&g, 20.0);
    let [sx, sz] = g
        .dry_land_near_m([x, z], 8000.0)
        .expect("land within reach");
    let h = g.surface_height_m(sx, sz);
    assert!(
        h >= g.water_level_m(sx, sz).unwrap() + 1.0,
        "{h} at {sx},{sz}"
    );
    for r in [48.0f32, 100.0] {
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            let (x, z) = (sx + r * a.cos(), sz + r * a.sin());
            assert!(
                g.water_surface_m(x, z).is_none(),
                "water {r} m from the spawn"
            );
        }
    }
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let near = g.surface_height_m(sx + 6.0 * a.cos(), sz + 6.0 * a.sin());
        assert!((near - h).abs() <= 4.0, "a cliff beside the spawn");
    }
}

#[test]
fn earlier_versions_spawn_where_they_always_did() {
    let v2 = WorldgenParams {
        generator: 2,
        ..wet_world()
    };
    let g = Worldgen::new(SEED, 1.0, v2);
    assert_eq!(g.dry_land_near_m([12.0, -7.0], 5000.0), Some([12.0, -7.0]));
}

fn caved_wet_v3() -> WorldgenParams {
    WorldgenParams {
        caves: Some(tesserae_core::CaveParams {
            radius_m: 0.001,
            ..tesserae_core::CaveParams::default()
        }),
        smoothing: None,
        ..wet_world()
    }
}

#[test]
fn caverns_under_the_water_table_are_underground_lakes_and_the_rest_are_air() {
    let g = Worldgen::new(SEED, 1.0, caved_wet_v3());
    let water = tesserae_core::testgen::fixtures().water;
    let (mut lake, mut dry_hollow, mut wet_above) = (0, 0, 0);
    for j in -16..16 {
        for i in -16..16 {
            let (x, z) = (i * 233i32, j * 241i32);
            let h = g.surface_height_m(x as f32, z as f32) as i32;
            if h < 130 {
                continue;
            }
            // From the band's floor up to well above the water table, deep under the ground.
            let (cx, cz) = (x.div_euclid(32), z.div_euclid(32));
            for y in -70i32..90 {
                let v = g.chunk(cx, y.div_euclid(32), cz).get(
                    x.rem_euclid(32) as usize,
                    y.rem_euclid(32) as usize,
                    z.rem_euclid(32) as usize,
                );
                if v.material() == water {
                    lake += 1;
                    wet_above += (y >= 22) as u32;
                } else if v.is_air() {
                    dry_hollow += (y >= 22) as u32;
                }
            }
        }
    }
    assert!(lake > 50, "{lake} underground lake voxels");
    assert_eq!(wet_above, 0, "water over the water table");
    assert!(
        dry_hollow > 50,
        "{dry_hollow} air voxels above the water table"
    );
}

fn fingerprint(chunk: &tesserae_core::Chunk) -> Vec<u16> {
    (0..CHUNK_SIZE)
        .flat_map(|z| (0..CHUNK_SIZE).flat_map(move |y| (0..CHUNK_SIZE).map(move |x| (x, y, z))))
        .map(|(x, y, z)| chunk.get(x, y, z).material())
        .collect()
}

#[test]
fn chunks_are_the_same_in_any_order_and_with_a_cold_cache() {
    let coords: Vec<(i32, i32, i32)> = (-3..3)
        .flat_map(|cx| (-2..2).flat_map(move |cz| (-3..4).map(move |cy| (cx * 37, cy, cz * 41))))
        .collect();
    let forward = Worldgen::new(SEED, 1.0, caved_wet_v3());
    let a: Vec<_> = coords
        .iter()
        .map(|&(x, y, z)| fingerprint(&forward.chunk(x, y, z)))
        .collect();
    let backward = Worldgen::new(SEED, 1.0, caved_wet_v3());
    let mut b: Vec<_> = coords
        .iter()
        .rev()
        .map(|&(x, y, z)| {
            backward.forget_regions();
            fingerprint(&backward.chunk(x, y, z))
        })
        .collect();
    b.reverse();
    assert_eq!(a, b);
}

#[test]
fn solid_and_air_are_the_same_in_metres_at_every_voxel_size() {
    let fine = Worldgen::new(SEED, 0.5, caved_wet_v3());
    let coarse = Worldgen::new(SEED, 1.0, caved_wet_v3());
    let at = |g: &Worldgen, size: f32, x_m: f32, y_m: f32, z_m: f32| {
        let (ix, iy, iz) = (
            (x_m / size) as i32,
            (y_m / size).floor() as i32,
            (z_m / size) as i32,
        );
        g.chunk(ix.div_euclid(32), iy.div_euclid(32), iz.div_euclid(32))
            .get(
                ix.rem_euclid(32) as usize,
                iy.rem_euclid(32) as usize,
                iz.rem_euclid(32) as usize,
            )
    };
    let mut compared = 0;
    for (x, z) in [
        (40.0, 40.0),
        (555.0, -310.0),
        (-1200.0, 900.0),
        (3000.0, 2500.0),
    ] {
        let h = coarse.surface_height_m(x, z);
        for dy in -30..30 {
            let y = (h + dy as f32).floor();
            let (a, b) = (at(&coarse, 1.0, x, y, z), at(&fine, 0.5, x, y, z));
            assert_eq!(a.is_air(), b.is_air(), "at {x},{y},{z}");
            compared += 1;
        }
    }
    assert!(compared > 200);
}

/// Columns where a cave once cut an overhang's rock above the ground under the water, leaving air.
#[test]
fn a_cave_cutting_an_overhang_under_water_leaves_water_not_air() {
    let params = WorldgenParams {
        smoothing: None,
        ..wet_world()
    };
    let g = Worldgen::new(1, 1.0, params);
    let mut wet = 0;
    for (x, z) in [
        (2988i32, 0i32),
        (-4067, -553),
        (-1743, 1896),
        (-3237, -3318),
        (-2656, -3239),
        (-3071, -632),
        (4067, 948),
        (2075, 2054),
    ] {
        let level = g.water_level_m(x as f32, z as f32).unwrap();
        let ground = g.surface_height_m(x as f32, z as f32);
        assert!(
            ground < level,
            "{x},{z} is no longer under water: retarget this test"
        );
        wet += 1;
        let from = ground.floor() as i32 + 1;
        let to = level.ceil() as i32;
        let air = air_column(&g, x, z, from, to);
        for (y, is_air) in (from..to).zip(air) {
            assert!(
                !(is_air && (y as f32) < level),
                "air under water at {x},{y},{z}"
            );
        }
    }
    assert_eq!(wet, 8);
}

#[test]
fn nothing_stands_above_a_footprints_own_top_and_versions_1_and_2_keep_the_global_bound() {
    let g = Worldgen::new(SEED, 1.0, caved_wet_v3());
    let (_, global) = g.surface_bounds_m();
    let mut lower = 0;
    for j in -12..12 {
        for i in -12..12 {
            let min = [i as f32 * 32.0 * 9.0, j as f32 * 32.0 * 11.0];
            let top = g.surface_top_m(min, [min[0] + 32.0, min[1] + 32.0]);
            assert!(top <= global);
            lower += (top < global - 40.0) as u32;
            let (cx, cz) = ((min[0] / 32.0) as i32, (min[1] / 32.0) as i32);
            let cy = (top / 32.0).floor() as i32 + 1;
            assert!(
                g.chunk(cx, cy, cz).uniform().is_some_and(|v| v.is_air()),
                "not sky over {min:?}, above {top}"
            );
        }
    }
    assert!(
        lower > 100,
        "{lower} footprints are well under the global bound"
    );
    let v2 = Worldgen::new(
        SEED,
        1.0,
        WorldgenParams {
            generator: 2,
            ..wet_world()
        },
    );
    assert_eq!(
        v2.surface_top_m([0.0, 0.0], [32.0, 32.0]),
        v2.surface_bounds_m().1
    );
}

/// Water that stands beside or above air would be poured into it by the fluid simulation.
#[test]
fn generated_water_has_no_air_below_or_beside_it() {
    let water = tesserae_core::testgen::fixtures().water;
    let params = WorldgenParams {
        caves: Some(tesserae_core::CaveParams::default()),
        smoothing: None,
        ..wet_world()
    };
    let g = Worldgen::new(SEED, 1.0, params);
    // 8x8 chunks wide, y -4..4 chunks
    let (x0, z0) = (-37i32, -41i32);
    let n = 16usize;
    let ys = -4..5;
    let mut grid = std::collections::HashMap::new();
    for cz in z0..z0 + n as i32 {
        for cx in x0..x0 + n as i32 {
            for cy in ys.clone() {
                grid.insert((cx, cy, cz), g.chunk(cx, cy, cz));
            }
        }
    }
    let get = |x: i32, y: i32, z: i32| {
        grid.get(&(x.div_euclid(32), y.div_euclid(32), z.div_euclid(32)))
            .map(|c| {
                c.get(
                    x.rem_euclid(32) as usize,
                    y.rem_euclid(32) as usize,
                    z.rem_euclid(32) as usize,
                )
            })
    };
    let (mut wet, mut down, mut side) = (0, 0, 0);
    for x in x0 * 32 + 1..(x0 + n as i32) * 32 - 1 {
        for z in z0 * 32 + 1..(z0 + n as i32) * 32 - 1 {
            for y in -128..128 {
                let Some(v) = get(x, y, z) else { continue };
                if v.is_air() || v.material() != water {
                    continue;
                }
                wet += 1;
                if get(x, y - 1, z).is_some_and(|b| b.is_air()) {
                    down += 1;
                }
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if get(x + dx, y, z + dz).is_some_and(|b| b.is_air()) {
                        side += 1;
                    }
                }
            }
        }
    }
    assert!(wet > 100_000, "the region holds water");
    assert_eq!(
        (down, side),
        (0, 0),
        "water would flow into {down} cells below and {side} beside it"
    );
}
