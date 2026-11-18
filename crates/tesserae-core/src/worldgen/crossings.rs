//! Crossings: a Template that spans stands only where its span crosses a gap, a valley with or
//! without water in it, from one dry landing to the other (ADR-0029). A random point seldom lands
//! on one, so a lattice cell searches its own ground and keeps the deepest gap it finds.

use super::fields::mix;
use super::ground::SAMPLE_M;
use super::structures::{INSET_M, Row, Structure};
use super::{Worldgen, top_index};
use crate::template::{LANDING_M, Template, Turn};

/// How far apart a cell's search tries the middle of a span.
const SEARCH_M: f32 = 3.0;
/// How far below the landings' Ground layer the dry ground under the middle third of a span lies
/// at least. A fixed depth, not one from the Template's deck, so a low deck is no excuse to bridge
/// a dip.
const GAP_M: f32 = 2.0;
/// Where water rather than a drop makes the gap, how deep it is at its deepest: every Voxel size
/// up to this shows some. A metre would show it at every size, but almost no crossing of gently
/// rolling ground is so deep.
const DEPTH_M: f32 = 0.5;

impl Worldgen {
    /// The deepest gap a held cell's search finds, its footprint inside the cell by [`INSET_M`].
    /// Equal depths go to the first tried, in an order from the cell's hash, so the answer is
    /// the same from every Chunk.
    pub(super) fn crossing(&self, row: &Row, cell: [i32; 2]) -> Option<Structure> {
        let h = self.held(row, cell)?;
        let template = &self.templates[usize::from(row.entry.template)];
        let spacing = row.entry.spacing_m;
        let low = cell.map(|c| c as f32 * spacing + INSET_M);
        let high = cell.map(|c| (c + 1) as f32 * spacing - INSET_M);
        let steps =
            |a: usize| (low[a] / SEARCH_M).ceil() as i32..=(high[a] / SEARCH_M).floor() as i32;
        let mut middles: Vec<[i32; 2]> = steps(1)
            .flat_map(|j| steps(0).map(move |i| [i, j]))
            .collect();
        for k in (1..middles.len()).rev() {
            let pick = (mix(h ^ (7 + k as u64)) % (k as u64 + 1)) as usize;
            middles.swap(k, pick);
        }
        let mut heights = Heights::new(low, high);
        let first = (mix(h ^ 2) & 3) as u8;
        let mut best: Option<(f32, Structure)> = None;
        let level_m = |p: [f32; 2]| self.water_level_m(p[0], p[1]).unwrap_or(f32::MIN);
        for step in middles {
            let middle_m = step.map(|s| s as f32 * SEARCH_M);
            let middle_h = heights.at(self, middle_m);
            for q in 0..4 {
                let turn = Turn::new(first + q);
                let along_m = turned_m(template, turn);
                let corner_m = [0, 1].map(|a| middle_m[a] - (along_m[a] / 2.0).floor());
                let inside =
                    (0..2).all(|a| corner_m[a] >= low[a] && corner_m[a] + along_m[a] <= high[a]);
                if !inside {
                    continue;
                }
                // The middle is in the middle third, and every landing is within the tolerance
                // of each far end: so a try whose ends are wet, or whose middle is neither wet nor
                // low enough under both, cannot take.
                let a = span_axis(template, turn);
                let last_m = ((along_m[a] - SAMPLE_M) / SAMPLE_M).ceil() * SAMPLE_M;
                let ends = [0.0, last_m].map(|k_m| {
                    let mut p = corner_m;
                    p[a] += k_m;
                    (p, heights.at(self, p))
                });
                let low_enough = middle_h < level_m(middle_m)
                    || middle_h <= ends[0].1.min(ends[1].1) + row.level_tolerance_m - GAP_M;
                if !low_enough || ends.iter().any(|&(p, h)| h < level_m(p)) {
                    continue;
                }
                if let Some((depth_m, s)) = self.gap(row, template, turn, corner_m, &mut heights)
                    && best
                        .as_ref()
                        .is_none_or(|(deepest_m, _)| depth_m > *deepest_m)
                {
                    best = Some((depth_m, s));
                }
            }
        }
        best.map(|(_, s)| s)
    }

    /// The Structure turned by `turn` with its corner at `corner_m`, and how deep its gap is, if
    /// it crosses one there: dry landings within its level tolerance; under the middle third of
    /// its span, ground at least [`GAP_M`] below them or water, [`DEPTH_M`] deep somewhere; no
    /// ground under the span rising into its deck; no cave under any of it; and clear of the
    /// origin. The rules are asked cheapest first.
    fn gap(
        &self,
        row: &Row,
        template: &Template,
        turn: Turn,
        corner_m: [f32; 2],
        heights: &mut Heights,
    ) -> Option<(f32, Structure)> {
        let along_m = turned_m(template, turn);
        let a = span_axis(template, turn);
        let span_m = along_m[a];
        let offsets = |side_m: f32| {
            (0..)
                .map(|k| k as f32 * SAMPLE_M)
                .take_while(move |&k_m| k_m < side_m)
        };
        let at = |k_m: f32, j_m: f32| {
            let mut p = corner_m;
            p[a] += k_m;
            p[1 - a] += j_m;
            p
        };
        let columns = |k: &dyn Fn(f32) -> bool| {
            offsets(span_m)
                .filter(|&k_m| k(k_m))
                .flat_map(|k_m| offsets(along_m[1 - a]).map(move |j_m| at(k_m, j_m)))
                .collect::<Vec<_>>()
        };
        let landing = |k_m: f32| k_m < LANDING_M || k_m >= span_m - LANDING_M;
        let level_m = |p: [f32; 2]| self.water_level_m(p[0], p[1]).unwrap_or(f32::MIN);

        let landings = columns(&landing);
        let (mut lowest_m, mut highest_m) = (f32::MAX, f32::MIN);
        let mut landing_m = Vec::with_capacity(landings.len());
        for &p in &landings {
            let height_m = heights.at(self, p);
            (lowest_m, highest_m) = (lowest_m.min(height_m), highest_m.max(height_m));
            if height_m < level_m(p) || highest_m - lowest_m > row.level_tolerance_m {
                return None;
            }
            landing_m.push(height_m);
        }
        landing_m.sort_by(f32::total_cmp);
        let median_m = landing_m[(landing_m.len() - 1) / 2];

        let (mut depth_m, mut wet_only, mut bed_m, mut bed_level_m) =
            (f32::MAX, false, f32::MAX, f32::MIN);
        for p in columns(&|k_m| (span_m / 3.0..2.0 * span_m / 3.0).contains(&k_m)) {
            let height_m = heights.at(self, p);
            let dropped = median_m - height_m >= GAP_M;
            if !dropped && height_m >= level_m(p) {
                return None;
            }
            wet_only |= !dropped;
            if height_m < bed_m {
                (bed_m, bed_level_m) = (height_m, level_m(p));
            }
            depth_m = depth_m.min(median_m - height_m);
        }
        if wet_only && bed_m > bed_level_m - DEPTH_M {
            return None;
        }
        // The higher landing's bank may stand above the Ground layer by the level tolerance, and
        // the ground beside it with it.
        let deck_m = highest_m + row.underside_m?;
        let span = columns(&|k_m| !landing(k_m));
        if span.iter().any(|&p| heights.at(self, p) > deck_m) {
            return None;
        }
        let listed = &self.structures.as_ref()?.listed;
        let unlisted = landings.iter().any(|p| {
            self.biome_at(p[0], p[1])
                .is_none_or(|b| listed[b.index()] & 1 << row.entry.template == 0)
        });
        if unlisted
            || landings
                .iter()
                .chain(&span)
                .any(|&p| self.cave_may_cut(p[0], p[1], heights.at(self, p)))
        {
            return None;
        }
        let corner = corner_m.map(|m| (m / self.voxel_size).round() as i32);
        let s = Structure {
            template: row.entry.template,
            turn,
            min: [
                corner[0],
                top_index(median_m, 0.0, self.voxel_size) - row.ground_top,
                corner[1],
            ],
            size: row.sizes[usize::from(turn.quarters())],
        };
        self.clear_of_origin(&s).then_some((depth_m, s))
    }

    /// Whether the column `[x, z]` of a spanning Structure is under one of its landings.
    pub(super) fn on_landing(&self, s: &Structure, [x, z]: [i32; 2]) -> bool {
        let template = &self.templates[usize::from(s.template)];
        let a = span_axis(template, s.turn);
        let offset_m = ([x, z][a] - [s.min[0], s.min[2]][a]) as f32 * self.voxel_size;
        offset_m < LANDING_M || offset_m >= template.span_m() - LANDING_M
    }
}

/// The surface heights at the whole metres of a lattice cell, each asked once per search.
struct Heights {
    min: [i32; 2],
    side: [i32; 2],
    /// NaN until asked.
    metres: Vec<f32>,
}

impl Heights {
    fn new(low: [f32; 2], high: [f32; 2]) -> Self {
        let min = low.map(|m| m.floor() as i32);
        let side = [0, 1].map(|a| high[a].ceil() as i32 - min[a] + 1);
        Heights {
            min,
            side,
            metres: vec![f32::NAN; (side[0] * side[1]) as usize],
        }
    }

    fn at(&mut self, worldgen: &Worldgen, p: [f32; 2]) -> f32 {
        let [x, z] = [0, 1].map(|a| p[a].round() as i32 - self.min[a]);
        let i = (z * self.side[0] + x) as usize;
        let height_m = &mut self.metres[i];
        if height_m.is_nan() {
            *height_m = worldgen.surface_height_m(p[0], p[1]);
        }
        *height_m
    }
}

/// The Template's sides along x and z, in metres, after `turn`.
fn turned_m(template: &Template, turn: Turn) -> [f32; 2] {
    let [x_m, _, z_m] = template.size_m();
    if turn.quarters().is_multiple_of(2) {
        [x_m, z_m]
    } else {
        [z_m, x_m]
    }
}

/// The World axis, 0 for x and 1 for z, its span runs along after `turn`.
fn span_axis(template: &Template, turn: Turn) -> usize {
    let [x_m, _, z_m] = template.size_m();
    usize::from(z_m > x_m) ^ usize::from(turn.quarters() % 2)
}
