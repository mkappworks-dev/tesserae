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
        assert_eq!(a.terrain_height_m(x, z), b.terrain_height_m(x, z));
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
            // A lip that holds water back stands a Voxel wide, so a coarse one may hold the shore.
            let lipped = |g: &Worldgen| g.surface_height_m(x, z) > g.terrain_height_m(x, z);
            if lipped(&fine) || lipped(&coarse) {
                continue;
            }
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
    // A range's upper flanks and summits: the highest rugged land, not a valley floor in it.
    let mut high: Vec<(f32, (i32, i32))> = (-80..80)
        .flat_map(|j| (-80..80).map(move |i| (i * 61, j * 61)))
        .filter(|&(x, z)| g.landform_fields(x as f32, z as f32).unwrap().ruggedness > 0.6)
        .map(|(x, z)| (g.surface_height_m(x as f32, z as f32), (x, z)))
        .collect();
    high.sort_by(|a, b| b.0.total_cmp(&a.0));
    let rugged: Vec<(i32, i32)> = high.iter().take(60).map(|&(_, c)| c).collect();
    assert!(rugged.len() >= 20, "{} rugged columns", rugged.len());
    let with = rugged
        .iter()
        .filter(|&&(x, z)| {
            let h = g.surface_height_m(x as f32, z as f32);
            has_overhang(&g, x, z, h as i32 - 14, h as i32 + 14)
        })
        .count();
    // Overhangs are scattered outcrops on a range's upper flanks and crests, not every slope.
    assert!(with >= 2, "{with} of {}", rugged.len());
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
            assert_eq!(fine.terrain_height_m(x, z), coarse.terrain_height_m(x, z));
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
fn a_world_with_no_settlement_spawns_on_the_dry_land_nearest_the_origin() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    // Recorded from the Server's search before the Generator answered it.
    assert_eq!(
        g.spawn_m().map(f32::to_bits),
        [3_286_458_261, 1_145_715_809]
    );
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
    for j in -32..32 {
        for i in -32..32 {
            let (x, z) = (i * 233i32, j * 241i32);
            let h = g.surface_height_m(x as f32, z as f32) as i32;
            if h < 60 {
                continue;
            }
            // From the band's floor up to the caverns' roof, deep under the ground.
            let (cx, cz) = (x.div_euclid(32), z.div_euclid(32));
            let table_m = g.water_table_m(x as f32, z as f32);
            for y in -70i32..h - 22 {
                let v = g.chunk(cx, y.div_euclid(32), cz).get(
                    x.rem_euclid(32) as usize,
                    y.rem_euclid(32) as usize,
                    z.rem_euclid(32) as usize,
                );
                if v.material() == water {
                    lake += 1;
                    wet_above += (y as f32 >= table_m) as u32;
                } else if v.is_air() {
                    dry_hollow += (y as f32 >= table_m) as u32;
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

/// Water Voxels with Air below or beside them in the `n`-Chunk square around Chunk column
/// `centre`, from `ys` Chunks: what the fluid simulation would pour out.
fn leaks(g: &Worldgen, centre: (i32, i32), n: i32, ys: std::ops::Range<i32>) -> (u32, u32, u32) {
    let water = tesserae_core::testgen::fixtures().water;
    let (x0, z0) = (centre.0 - n / 2, centre.1 - n / 2);
    let mut grid = std::collections::HashMap::new();
    for cz in z0..z0 + n {
        for cx in x0..x0 + n {
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
    for x in x0 * 32 + 1..(x0 + n) * 32 - 1 {
        for z in z0 * 32 + 1..(z0 + n) * 32 - 1 {
            for y in ys.start * 32 + 1..ys.end * 32 {
                let Some(v) = get(x, y, z) else { continue };
                if v.material() != water {
                    continue;
                }
                wet += 1;
                down += get(x, y - 1, z).is_some_and(|b| b.is_air()) as u32;
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    side += get(x + dx, y, z + dz).is_some_and(|b| b.is_air()) as u32;
                }
            }
        }
    }
    (wet, down, side)
}

fn caved_world() -> WorldgenParams {
    WorldgenParams {
        caves: Some(tesserae_core::CaveParams::default()),
        smoothing: None,
        ..wet_world()
    }
}

/// Water that stands beside or above air would be poured into it by the fluid simulation.
#[test]
fn generated_water_has_no_air_below_or_beside_it() {
    let g = Worldgen::new(SEED, 1.0, caved_world());
    let (wet, down, side) = leaks(&g, (-29, -33), 16, -4..5);
    assert!(wet > 100_000, "the region holds water");
    assert_eq!(
        (down, side),
        (0, 0),
        "water would flow into {down} cells below and {side} beside it"
    );
}

/// The first `want` Chunk columns, on a coarse walk, whose water stands above the sea.
fn inland_water_chunks(g: &Worldgen, want: usize) -> Vec<(i32, i32)> {
    let mut found: Vec<(i32, i32)> = Vec::new();
    for j in -40..40 {
        for i in -40..40 {
            let (x, z) = (i as f32 * 61.0, j as f32 * 67.0);
            let wet = g.water_surface_m(x, z).is_some_and(|l| l > 25.0);
            let chunk = ((x / 32.0).floor() as i32, (z / 32.0).floor() as i32);
            let apart = found
                .iter()
                .all(|c| (c.0 - chunk.0).abs() > 8 || (c.1 - chunk.1).abs() > 8);
            if wet && apart {
                found.push(chunk);
                if found.len() == want {
                    return found;
                }
            }
        }
    }
    found
}

#[test]
fn lakes_and_rivers_meet_their_shores_and_each_other_without_a_wall() {
    let g = Worldgen::new(SEED, 1.0, caved_world());
    let mut total = 0;
    for centre in inland_water_chunks(&g, 6) {
        let (wet, down, side) = leaks(&g, centre, 6, 0..4);
        total += wet;
        assert_eq!(
            (down, side),
            (0, 0),
            "around Chunk column {centre:?}, water would flow into {down} cells below and {side} beside it"
        );
    }
    assert!(total > 10_000, "{total} inland water Voxels");
}

#[test]
fn flooded_caverns_are_sealed_from_dry_tunnels() {
    let g = Worldgen::new(SEED, 1.0, caved_world());
    // Under the mountains, where caverns are deep and tunnels cross them below the water table.
    let mut checked = 0;
    for j in -32..32 {
        for i in -32..32 {
            let (x, z) = (i * 233i32, j * 241i32);
            if g.surface_height_m(x as f32, z as f32) < 90.0 || checked == 4 {
                continue;
            }
            checked += 1;
            let (_, down, side) = leaks(&g, (x.div_euclid(32), z.div_euclid(32)), 4, -3..1);
            assert_eq!(
                (down, side),
                (0, 0),
                "under {x}, {z}, cavern water would flow into {down} cells below and {side} beside it"
            );
        }
    }
    assert_eq!(checked, 4);
}

/// Over rugged ground: the mean bend of the surface between neighbouring metres, and the share of
/// columns that are level over 8 m.
fn rugged_shape(g: &Worldgen) -> (f32, f32, usize) {
    let (mut bend, mut level, mut n) = (0.0f32, 0usize, 0usize);
    for j in -60..60 {
        for i in -60..60 {
            let (x, z) = (i as f32 * 157.0, j as f32 * 163.0);
            if g.landform_fields(x, z).is_none_or(|f| f.ruggedness < 0.5) {
                continue;
            }
            let h = |dx: f32| g.terrain_height_m(x + dx, z);
            bend += (h(1.0) - 2.0 * h(0.0) + h(-1.0)).abs();
            level += ((h(4.0) - h(-4.0)).abs() < 0.8) as usize;
            n += 1;
        }
    }
    (bend / n as f32, level as f32 / n as f32, n)
}

#[test]
fn mountain_flanks_are_smooth_and_level_floors_lie_between_them() {
    let g = Worldgen::new(SEED, 1.0, v3());
    let (bend, level, n) = rugged_shape(&g);
    assert!(n > 500, "{n} rugged columns");
    // Mountains cut by calm patches and covered in fine knobs bent 0.038 m a metre.
    assert!(bend < 0.03, "the ground bends {bend} m a metre");
    assert!(level > 0.4, "{level} of rugged columns are level");
}

/// The share of rugged land whose ground stands under a lake or river: high where valleys between
/// mountains are closed basins the drainage fills.
fn rugged_under_inland_water(g: &Worldgen) -> f32 {
    let (mut wet, mut n) = (0usize, 0usize);
    for j in -60..60 {
        for i in -60..60 {
            let (x, z) = (i as f32 * 157.0, j as f32 * 163.0);
            if g.landform_fields(x, z).is_none_or(|f| f.ruggedness < 0.5) {
                continue;
            }
            n += 1;
            wet += g.water_surface_m(x, z).is_some_and(|l| l > 24.5) as usize;
        }
    }
    wet as f32 / n as f32
}

#[test]
fn valleys_between_mountains_drain_rather_than_flood() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let wet = rugged_under_inland_water(&g);
    // Valleys that were the blobs between ridges flooded 9% of it.
    assert!(wet < 0.07, "{wet} of rugged land is under a lake");
}

/// Over the 60 highest rugged columns: the share whose ground slopes steeper than 1 in 2 across
/// 4 m, and the share whose ground rises and falls more than 2 m within 6 m.
fn summit_shape(g: &Worldgen) -> (f32, f32) {
    let mut high: Vec<(f32, (f32, f32))> = (-80..80)
        .flat_map(|j| (-80..80).map(move |i| (i as f32 * 61.0, j as f32 * 61.0)))
        .filter(|&(x, z)| g.landform_fields(x, z).unwrap().ruggedness > 0.6)
        .map(|(x, z)| (g.terrain_height_m(x, z), (x, z)))
        .collect();
    high.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (mut steep, mut rough) = (0, 0);
    for &(_, (x, z)) in high.iter().take(60) {
        let h = |dx: f32, dz: f32| g.terrain_height_m(x + dx, z + dz);
        let slope = ((h(2.0, 0.0) - h(-2.0, 0.0)).powi(2) + (h(0.0, 2.0) - h(0.0, -2.0)).powi(2))
            .sqrt()
            / 4.0;
        steep += (slope > 0.5) as usize;
        let ring: Vec<f32> = (-3..=3)
            .flat_map(|j| (-3..=3).map(move |i| (i as f32 * 2.0, j as f32 * 2.0)))
            .map(|(dx, dz)| h(dx, dz))
            .collect();
        let (lo, hi) = ring
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, u), &v| (l.min(v), u.max(v)));
        rough += (hi - lo > 2.0) as usize;
    }
    (steep as f32 / 60.0, rough as f32 / 60.0)
}

#[test]
fn mountains_have_steep_upper_flanks_and_uneven_summits() {
    let (steep, rough) = summit_shape(&Worldgen::new(SEED, 1.0, v3()));
    // Rounded massifs had 3% of their highest ground this steep and half of it uneven.
    assert!(steep > 0.2, "{steep} of summit columns are steep");
    assert!(rough > 0.8, "{rough} of summit columns are uneven");
}

/// Over mountain land (rugged, well above its valley): the share of columns sloping more than
/// 1 in 1 across 2 m, and of those that are a wall, rising more than 3 m within a metre.
fn mountain_slopes(g: &Worldgen) -> (f32, f32, f32, usize) {
    let (mut steep, mut wall, mut bench, mut n) = (0, 0, 0, 0);
    for j in -120..120 {
        for i in -120..120 {
            let (x, z) = (i as f32 * 41.0, j as f32 * 43.0);
            if g.landform_fields(x, z).is_none_or(|f| f.ruggedness < 0.5) {
                continue;
            }
            let h = |dx: f32, dz: f32| g.terrain_height_m(x + dx, z + dz);
            if h(0.0, 0.0) < 24.0 + 40.0 {
                continue;
            }
            n += 1;
            let g2 = ((h(1.0, 0.0) - h(-1.0, 0.0)).powi(2) + (h(0.0, 1.0) - h(0.0, -1.0)).powi(2))
                .sqrt()
                / 2.0;
            steep += (g2 > 1.0) as usize;
            let rise = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
                .iter()
                .map(|&(dx, dz)| (h(dx, dz) - h(0.0, 0.0)).abs())
                .fold(0.0f32, f32::max);
            wall += (rise > 3.0) as usize;
            bench += (g2 < 0.15) as usize;
        }
    }
    let f = |k: usize| k as f32 / n as f32;
    (f(steep), f(wall), f(bench), n)
}

#[test]
fn mountains_are_climbable_with_benches_and_few_walls() {
    let (_, wall, bench, n) = mountain_slopes(&Worldgen::new(SEED, 1.0, v3()));
    assert!(n > 200, "{n} mountain columns");
    assert_eq!(wall, 0.0, "the heightfield has no walls");
    // Smooth flanks had a fifth of their ground level; benches make it more.
    assert!(bench > 0.25, "{bench} of mountain ground is level");
    // Overhangs everywhere on rugged ground made a wall at 6% of mountain columns.
    let (walls, n) = voxel_walls(&Worldgen::new(SEED, 1.0, dry_v3()));
    assert!(n > 30, "{n} mountain columns");
    assert!(walls < 0.04, "{walls} of mountain columns stand by a wall");
}

/// From generated Chunks over mountain land: the share of columns whose top solid Voxel stands
/// more than 3 Voxels above or below a neighbour's.
fn voxel_walls(g: &Worldgen) -> (f32, usize) {
    let (mut walls, mut n) = (0, 0);
    for j in -40..40 {
        for i in -40..40 {
            let (x, z) = (i * 123, j * 129);
            if g.landform_fields(x as f32, z as f32)
                .is_none_or(|f| f.ruggedness < 0.5)
                || g.terrain_height_m(x as f32, z as f32) < 24.0 + 40.0
            {
                continue;
            }
            let top = |x: i32, z: i32| {
                let h = g.terrain_height_m(x as f32, z as f32) as i32;
                (h - 16..h + 16)
                    .rev()
                    .find(|&y| {
                        !g.chunk(x.div_euclid(32), y.div_euclid(32), z.div_euclid(32))
                            .get(
                                x.rem_euclid(32) as usize,
                                y.rem_euclid(32) as usize,
                                z.rem_euclid(32) as usize,
                            )
                            .is_air()
                    })
                    .unwrap_or(h - 16)
            };
            n += 1;
            let here = top(x, z);
            walls += [(1, 0), (0, 1)]
                .iter()
                .any(|&(dx, dz)| (top(x + dx, z + dz) - here).abs() > 3)
                as usize;
        }
    }
    (walls as f32 / n as f32, n)
}

#[test]
fn rock_formations_stand_on_the_sea_floor_and_stay_under_water() {
    let g = Worldgen::new(SEED, 1.0, v3());
    let (mut ocean, mut formations) = (0, 0);
    for j in -150..150 {
        for i in -150..150 {
            let (x, z) = (i as f32 * 17.0, j as f32 * 19.0);
            let h = |dx: f32, dz: f32| g.terrain_height_m(x + dx, z + dz);
            let here = h(0.0, 0.0);
            if here >= 24.0 - 1.0 || here < 24.0 - 40.0 {
                continue;
            }
            ocean += 1;
            let around = [(12.0, 0.0), (-12.0, 0.0), (0.0, 12.0), (0.0, -12.0)]
                .map(|(dx, dz)| h(dx, dz))
                .into_iter()
                .fold(f32::MAX, f32::min);
            // Rising off a floor 8 m or more down: a formation, not the shore's own slope.
            if here - around > 3.0 && around < 24.0 - 8.0 {
                formations += 1;
                assert!(
                    here <= 24.0 - 2.0 + 1e-3,
                    "a formation at {x},{z} reaches {here}"
                );
            }
        }
    }
    assert!(ocean > 5000, "{ocean} sea columns");
    assert!(
        formations * 200 > ocean,
        "{formations} formations in {ocean} sea columns"
    );
}

/// Land never jumps between two points a quarter of a metre apart: a cliff the heightfield draws
/// is a slope, and only the overhang density stands walls. The sea floor's pillars are steep on
/// purpose.
#[test]
fn the_ground_has_no_step_anywhere_along_a_line() {
    let g = Worldgen::new(SEED, 1.0, wet_world());
    let mut worst = (0.0f32, 0.0f32, 0.0f32);
    for j in -200..200 {
        let z = j as f32 * 19.0 + 0.3;
        let mut last = g.terrain_height_m(-4000.0, z);
        for i in 1..32000 {
            let x = -4000.0 + i as f32 * 0.25;
            let h = g.terrain_height_m(x, z);
            if (h - last).abs() > worst.0 && h.min(last) > 24.0 {
                worst = ((h - last).abs(), x, z);
            }
            last = h;
        }
    }
    assert!(
        worst.0 < 1.0,
        "the ground steps {} m at {}, {}",
        worst.0,
        worst.1,
        worst.2
    );
}
