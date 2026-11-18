//! The fixtures' content set: every kind of content generation places, each shown standing.

#![allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]

use std::collections::HashSet;

use tesserae_core::testgen::{content, fixtures};
use tesserae_core::{CHUNK_SIZE, Chunk, Fluid, MaterialId, Worldgen};

/// The golden digests' seeds and Voxel sizes.
const CASES: [(u64, f32); 4] = [
    (0x7e55_e2ae_0000_0001, 1.0),
    (0x7e55_e2ae_0000_0001, 0.5),
    (0xdead_beef_cafe_f00d, 1.0),
    (0xdead_beef_cafe_f00d, 0.5),
];

/// The Chunks within `half_m` of `centre_m`, from 16 m under the ground to 16 m over it.
fn chunks_near(g: &Worldgen, centre_m: [f32; 2], half_m: f32) -> impl Iterator<Item = Chunk> + '_ {
    let chunk_m = CHUNK_SIZE as f32 * g.voxel_size();
    let span = move |c: f32| {
        ((c - half_m) / chunk_m).floor() as i32..=((c + half_m) / chunk_m).floor() as i32
    };
    span(centre_m[1]).flat_map(move |cz| {
        span(centre_m[0]).flat_map(move |cx| {
            let h = g.surface_height_m((cx as f32 + 0.5) * chunk_m, (cz as f32 + 0.5) * chunk_m);
            let rows =
                ((h - 16.0) / chunk_m).floor() as i32..=((h + 16.0) / chunk_m).floor() as i32;
            rows.map(move |cy| g.chunk(cx, cy, cz))
        })
    })
}

/// The water at least 2 m deep nearest `from`, searched in rings 16 m apart: a reed leaves a Voxel
/// of water over its top.
fn water_near(g: &Worldgen, from: [f32; 2]) -> [f32; 2] {
    (0..128)
        .flat_map(|ring| {
            let r = ring as f32 * 16.0;
            let n = (ring * 6).max(1);
            (0..n).map(move |k| {
                let a = k as f32 * std::f32::consts::TAU / n as f32;
                [from[0] + r * a.cos(), from[1] + r * a.sin()]
            })
        })
        .find(|&[x, z]| {
            g.water_surface_m(x, z)
                .is_some_and(|level_m| level_m - g.surface_height_m(x, z) >= 2.0)
        })
        .expect("water within 2 km")
}

fn materials_near(g: &Worldgen, centre_m: [f32; 2], half_m: f32) -> HashSet<MaterialId> {
    chunks_near(g, centre_m, half_m)
        .flat_map(|c| c.kinds().iter().map(|v| v.material()).collect::<Vec<_>>())
        .collect()
}

#[test]
fn the_content_biome_stands_its_surface_feature() {
    for (seed, size) in CASES {
        let g = content().worldgen(seed, size);
        let found = materials_near(&g, g.spawn_m(), 128.0);
        assert!(
            found.contains(&fixtures().content.flower),
            "{seed:x} at {size} m"
        );
    }
}

#[test]
fn the_column_plant_stands_in_deep_water_and_carries_it() {
    let water = fixtures().water;
    let reed = fixtures().content.reed;
    for (seed, size) in CASES {
        let g = content().worldgen(seed, size);
        let wet = chunks_near(&g, water_near(&g, g.spawn_m()), 64.0).any(|chunk| {
            let overlay = content().plants.generated_overlay(&chunk, Some(water));
            (0..CHUNK_SIZE.pow(3)).any(|i| {
                let [x, y, z] = [
                    i % CHUNK_SIZE,
                    i / CHUNK_SIZE % CHUNK_SIZE,
                    i / CHUNK_SIZE.pow(2),
                ];
                chunk.get(x, y, z).material() == reed
                    && overlay.get(x, y, z).fluid == Some(Fluid::source(water))
            })
        });
        assert!(wet, "{seed:x} at {size} m: no reed standing in water");
    }
}

#[test]
fn a_crop_field_stands_near_the_water_with_its_lanes() {
    let m = &fixtures().content;
    for (seed, size) in CASES {
        let g = content().worldgen(seed, size);
        let found = materials_near(&g, g.spawn_m(), 128.0);
        assert!(
            m.crop.iter().any(|c| found.contains(c)),
            "{seed:x} at {size} m: no crop"
        );
        assert!(found.contains(&m.lane), "{seed:x} at {size} m: no lane");
    }
}

#[test]
fn a_structure_stands_in_its_templates_material() {
    for (seed, size) in CASES {
        let g = content().worldgen(seed, size);
        let found = materials_near(&g, g.spawn_m(), 128.0);
        assert!(
            found.contains(&fixtures().content.stone),
            "{seed:x} at {size} m"
        );
    }
}

#[test]
fn the_near_spawn_settlement_is_where_a_player_spawns() {
    let m = &fixtures().content;
    for (seed, size) in CASES {
        let g = content().worldgen(seed, size);
        let spawn = g.settlement_spawn_m(4096.0);
        assert_eq!(
            spawn,
            Some(g.spawn_m()),
            "{seed:x} at {size} m: no Settlement to spawn in"
        );
        let found = materials_near(&g, g.spawn_m(), content().settlements[0].reach_m());
        assert!(found.contains(&m.hall), "{seed:x} at {size} m: no building");
        assert!(found.contains(&m.road), "{seed:x} at {size} m: no road");
    }
}
