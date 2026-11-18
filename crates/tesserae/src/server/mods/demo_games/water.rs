//! Water in a Terra World: no air standing inside it, and no Plant punching a hole in its top.

use tesserae_core::{Voxel, Worldgen, WorldgenParams};

use super::{loaded, material};

/// Every Voxel of the metre column `x, z` from `from_m` up to `to_m`.
fn column(g: &Worldgen, x: i32, z: i32, from_m: i32, to_m: i32) -> Vec<Voxel> {
    let (cx, cz) = (x.div_euclid(32), z.div_euclid(32));
    let (lx, lz) = (x.rem_euclid(32) as usize, z.rem_euclid(32) as usize);
    let mut out = Vec::new();
    let mut y = from_m;
    while y < to_m {
        let cy = y.div_euclid(32);
        let chunk = g.chunk(cx, cy, cz);
        while y < to_m && y.div_euclid(32) == cy {
            out.push(chunk.get(lx, y.rem_euclid(32) as usize, lz));
            y += 1;
        }
    }
    out
}

#[test]
fn version_3_water_has_no_air_in_it_and_kelp_leaves_the_surface_whole() {
    let (_install, loaded) = loaded("terra", 1.0);
    let world = loaded.world.unwrap();
    let params = WorldgenParams {
        generator: 3,
        ..world
    };
    let kelp = material(&loaded, "aquatic:kelp");
    let g = Worldgen::new(1, 1.0, params);
    let (mut wet, mut kelp_columns, mut pockets, mut holed_tops) = (0, 0, 0, 0);
    // Every wet column generates its own Chunks, nearly all of the test's time: one in four of the
    // columns 59 by 61 m apart, over the same 12 km square, still finds 1803 wet and 53 with kelp.
    for j in -50..50 {
        for i in -50..50 {
            let (x, z) = (i * 118, j * 122);
            let Some(level) = g.water_level_m(x as f32, z as f32) else {
                continue;
            };
            let ground = g.surface_height_m(x as f32, z as f32);
            if ground >= level {
                continue;
            }
            wet += 1;
            let from = ground.floor() as i32 + 1;
            let to = level.ceil() as i32;
            let cells = column(&g, x, z, from, to);
            // The cells whose lower corner is under the water.
            let under: Vec<_> = (from..to)
                .zip(&cells)
                .filter(|(y, _)| (*y as f32) < level)
                .map(|(_, v)| *v)
                .collect();
            pockets += under.iter().filter(|v| v.is_air()).count();
            if under.iter().any(|v| v.material() == kelp) {
                kelp_columns += 1;
                // Overhanging rock may stand at the surface; kelp may not.
                let holed = under.last().is_some_and(|v| v.material() == kelp);
                holed_tops += usize::from(holed);
            }
        }
    }
    assert!(
        wet > 200 && kelp_columns > 20,
        "{wet} wet and {kelp_columns} kelp columns"
    );
    assert_eq!(pockets, 0, "air standing inside water");
    assert_eq!(holed_tops, 0, "a Plant where the surface water should be");
}
