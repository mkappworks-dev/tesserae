//! The fluid a point sits in or just above, in metres.
//!
//! Heights are metres (ADR-0003): a Voxel's `surface` is a fraction of its edge,
//! so the same lake reads the same at any Voxel size.

use crate::flow::CellView;
use crate::occupants::{Fluid, Occupants};
use crate::tables::Tables;
use crate::voxel::MaterialId;

/// The fluid at an eye's column and the height of its surface.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FluidSurface {
    pub material: MaterialId,
    pub surface_y_m: f32,
    /// The fluid Material's light dimming, which sets how far it can be seen
    /// through.
    pub dimming: u8,
}

/// The fluid at the eye's cell, or the highest within `reach_m` below it.
///
/// Stops at a solid or unheld cell, so a floor between the eye and a pool below
/// hides the pool. Fluid in the voids of a solid is not read: the eye is in the
/// open space beside it.
pub fn fluid_at(
    view: &impl CellView,
    tables: &Tables,
    size: f32,
    eye_m: [f32; 3],
    reach_m: f32,
) -> Option<FluidSurface> {
    let [x, eye_y, z] = eye_m.map(|m| (m / size).floor() as i32);
    // One cell lower: a cell ending at the reach is wet up to its top.
    let lowest = ((eye_m[1] - reach_m) / size).floor() as i32 - 1;
    let open = |y| {
        let (voxel, overlay) = view.stored([x, y, z])?;
        let here = Occupants::of(voxel, overlay, &tables.fluids);
        here.solid.is_air().then_some(here.fluid)
    };
    let mut y = eye_y;
    let mut fluid = loop {
        match open(y)? {
            Some(fluid) => break fluid,
            None if y > lowest => y -= 1,
            None => return None,
        }
    };
    while let Some(Some(above)) = open(y + 1) {
        fluid = above;
        y += 1;
    }
    let surface_y_m = (y as f32 + f32::from(fluid.surface) / f32::from(Fluid::FULL_SURFACE)) * size;
    (surface_y_m >= eye_m[1] - reach_m).then_some(FluidSurface {
        material: fluid.material,
        surface_y_m,
        dimming: tables.materials.def(fluid.material).dimming,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::Cell;
    use crate::occupants::{Fluid, Level, Overlay};
    use crate::shape::MAX_VOIDS;
    use crate::testgen::fixtures;
    use crate::voxel::Voxel;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Cells(HashMap<Cell, (Voxel, Overlay)>);

    impl CellView for Cells {
        fn stored(&self, at: Cell) -> Option<(Voxel, Overlay)> {
            self.0.get(&at).copied()
        }
    }

    /// A column of full water cells from `from` up to but not including `to`,
    /// over air to `top`; everything outside the column is unheld.
    fn lake(water: MaterialId, from: i32, to: i32, top: i32) -> Cells {
        let mut cells = Cells::default();
        for y in from..top {
            let voxel = if y < to {
                Voxel::cube(water)
            } else {
                Voxel::AIR
            };
            cells.0.insert([0, y, 0], (voxel, Overlay::NONE));
        }
        cells
    }

    #[test]
    fn an_eye_in_a_full_lake_reads_the_top_of_the_water() {
        let f = fixtures();
        let cells = lake(f.water, 0, 4, 8);
        let found = fluid_at(&cells, &f.tables, 1.0, [0.5, 2.5, 0.5], 2.0);
        assert_eq!(
            found,
            Some(FluidSurface {
                material: f.water,
                surface_y_m: 4.0,
                dimming: 2,
            })
        );
    }

    #[test]
    fn an_eye_above_the_water_reads_it_only_within_reach() {
        let f = fixtures();
        let cells = lake(f.water, 0, 4, 9);
        let at = |y| fluid_at(&cells, &f.tables, 1.0, [0.5, y, 0.5], 2.0);
        assert_eq!(at(5.5).map(|s| s.surface_y_m), Some(4.0));
        assert_eq!(at(6.0).map(|s| s.surface_y_m), Some(4.0));
        assert_eq!(at(6.5), None, "2.5 m above the surface is out of reach");
    }

    fn flowing(material: MaterialId, surface: u8) -> Overlay {
        let mut levels = [Level::DRY; MAX_VOIDS];
        levels[0] = Level {
            surface,
            reach: 1,
            descent: 0,
        };
        Overlay {
            fluid: Fluid::flowing(material, levels),
            burning: None,
        }
    }

    #[test]
    fn a_partial_surface_is_measured_in_metres_at_any_voxel_size() {
        let f = fixtures();
        for size in [0.25_f32, 0.5, 1.0] {
            let wet = (2.0 / size) as i32;
            let mut cells = lake(f.water, 0, wet, wet + 8);
            // Half the cell above the lake: the surface is 2 m plus half a Voxel.
            cells
                .0
                .insert([0, wet, 0], (Voxel::AIR, flowing(f.water, 8)));
            let at = |y| fluid_at(&cells, &f.tables, size, [0.1, y, 0.1], 2.0);
            let surface = 2.0 + size / 2.0;
            let found = at(1.0).unwrap().surface_y_m;
            assert!((found - surface).abs() < 1e-5, "{size}: {found}");
            assert!(at(surface + 0.01).is_some(), "just above, still in reach");
            assert!(at(surface + 2.01).is_none(), "{size}: out of reach");
        }
    }

    #[test]
    fn a_floor_between_the_eye_and_a_pool_hides_the_pool() {
        let f = fixtures();
        let mut cells = lake(f.water, 0, 2, 8);
        cells
            .0
            .insert([0, 3, 0], (Voxel::cube(f.ground.rock), Overlay::NONE));
        let at = fluid_at(&cells, &f.tables, 1.0, [0.5, 4.5, 0.5], 4.0);
        assert_eq!(at, None);
    }

    #[test]
    fn unheld_terrain_reads_as_no_fluid() {
        let f = fixtures();
        let cells = Cells::default();
        assert_eq!(fluid_at(&cells, &f.tables, 1.0, [0.5, 4.5, 0.5], 2.0), None);
    }
}
