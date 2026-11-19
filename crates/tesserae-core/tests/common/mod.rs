#![allow(
    dead_code,
    reason = "every test binary that declares this module compiles all of it and calls only some"
)]

use std::ops::RangeInclusive;

use tesserae_core::terrain::LandformParams;
use tesserae_core::{Worldgen, WorldgenParams};

/// The content digests' seeds and Voxel sizes. The sea stands over the origin at each.
pub const CASES: [(u64, f32); 4] = [
    (0x7e55_e2ae_0000_0001, 1.0),
    (0x7e55_e2ae_0000_0001, 0.5),
    (0xdead_beef_cafe_f00d, 1.0),
    (0xdead_beef_cafe_f00d, 0.5),
];

/// The land held between Sea level and `relief_m` above it, with no sea floor and no overhangs,
/// for a test of what stands on the ground rather than its shape.
pub fn lowland(params: WorldgenParams) -> WorldgenParams {
    WorldgenParams {
        landforms: LandformParams {
            depth_m: 0.0,
            height_m: params.relief_m,
            overhang_m: 0.0,
            overhangs: false,
            ..LandformParams::default()
        },
        ..params
    }
}

/// `params` with the sea floor let down `relief_m` under Sea level, so the sea stands on the
/// lowest of the land.
pub fn sea_floor(params: WorldgenParams) -> WorldgenParams {
    WorldgenParams {
        landforms: LandformParams {
            depth_m: params.relief_m,
            ..params.landforms
        },
        ..params
    }
}

/// The Chunk indices along one axis that `centre_m - half_m..=centre_m + half_m` crosses.
pub fn chunk_span(centre_m: f32, half_m: f32, chunk_m: f32) -> RangeInclusive<i32> {
    ((centre_m - half_m) / chunk_m).floor() as i32..=((centre_m + half_m) / chunk_m).floor() as i32
}

/// The water at least 2 m deep nearest `from`, searched in rings 16 m apart: a reed leaves a Voxel
/// of water over its top.
#[allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]
pub fn water_near(g: &Worldgen, from: [f32; 2]) -> [f32; 2] {
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
