//! The ground rules a Structure and a Plot share: columns sampled a metre apart, dry, their lower
//! median and no cave cutting their top metre. They ask only metres, so every Voxel size and every
//! Chunk decides alike.

use super::Worldgen;

/// How far apart the ground rules ask a footprint's columns.
pub(super) const SAMPLE_M: f32 = 1.0;

/// One sampled column of the ground.
#[derive(Clone, Copy, Debug)]
pub(super) struct Column {
    pub(super) x_m: f32,
    pub(super) z_m: f32,
    pub(super) height_m: f32,
    /// Under the footprint itself, not only the ground kept round it.
    pub(super) under: bool,
}

/// The columns a footprint's ground was sampled at, every one dry.
pub(super) struct Ground(Vec<Column>);

impl Ground {
    pub(super) fn columns(&self) -> impl Iterator<Item = &Column> {
        self.0.iter()
    }

    fn under(&self) -> impl Iterator<Item = &Column> {
        self.columns().filter(|c| c.under)
    }

    /// The lower median height of the columns under the footprint.
    pub(super) fn lower_median_m(&self) -> Option<f32> {
        let mut heights: Vec<f32> = self.under().map(|c| c.height_m).collect();
        heights.sort_by(f32::total_cmp);
        heights.get((heights.len().max(1) - 1) / 2).copied()
    }
}

impl Worldgen {
    /// The ground at `points`, each `([x_m, z_m], under)`, if every column is above the lake and
    /// `takes` each one; `None` at the first that is not.
    pub(super) fn sample_ground(
        &self,
        points: impl IntoIterator<Item = ([f32; 2], bool)>,
        mut takes: impl FnMut(&Column) -> bool,
    ) -> Option<Ground> {
        let mut columns = Vec::new();
        for ([x_m, z_m], under) in points {
            let height_m = self.surface_height_m(x_m, z_m);
            // Below the level some Voxel size floods the column, if not this one.
            if self
                .water_level_m(x_m, z_m)
                .is_some_and(|level_m| height_m < level_m)
            {
                return None;
            }
            let column = Column {
                x_m,
                z_m,
                height_m,
                under,
            };
            if !takes(&column) {
                return None;
            }
            columns.push(column);
        }
        Some(Ground(columns))
    }

    /// Whether no cave cuts the top metre of any column under the footprint. Asked last, as the
    /// dearest rule.
    pub(super) fn uncut(&self, ground: &Ground) -> bool {
        !ground
            .under()
            .any(|c| self.cave_may_cut(c.x_m, c.z_m, c.height_m))
    }

    /// Whether a cave may cut the surface of the column at `(x_m, z_m)`. Every Voxel size up to
    /// [`SAMPLE_M`] has its surface Voxel's corner, where caves are asked, in the metre under the
    /// surface, and `false` is a proof over that metre, so it holds at each of them.
    pub(super) fn cave_may_cut(&self, x_m: f32, z_m: f32, height_m: f32) -> bool {
        self.caves.as_ref().is_some_and(|caves| {
            let depth_m = self.voxel_size.max(SAMPLE_M);
            caves.may_carve(
                [x_m, height_m - depth_m, z_m],
                [x_m, height_m, z_m],
                self.cave_scale_at(x_m, z_m),
            )
        })
    }
}
