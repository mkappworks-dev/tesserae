//! Layout geometry: which Voxel coordinates a line, rectangle or box covers.
//!
//! Pure and in Voxel units only: a Voxel count never depends on the World's
//! Voxel size, so the Server and the Client's preview expand the same list.

/// Most Voxels one layout may cover; equals the Server's pending-queue bound.
pub const MAX_LAYOUT_VOXELS: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dim {
    X,
    Y,
    Z,
}

/// One axis of a layout: the dimension it grows along, which way, and how many
/// Voxels it holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LayoutAxis {
    pub dim: Dim,
    pub positive: bool,
    pub count: u32,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Layout {
    pub anchor: [i32; 3],
    /// One to three axes; the first varies fastest.
    pub axes: Vec<LayoutAxis>,
    /// Voxels left empty between neighbours along every axis.
    pub gap: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LayoutError {
    /// Not one to three axes.
    AxisCount,
    RepeatedDimension,
    EmptyAxis,
    /// More than [`MAX_LAYOUT_VOXELS`] Voxels.
    TooManyVoxels,
    /// A coordinate would leave `i32`.
    OutOfRange,
}

/// The Voxel coordinates a layout covers, first axis fastest.
pub fn layout_cells(layout: &Layout) -> Result<Vec<[i32; 3]>, LayoutError> {
    if !(1..=3).contains(&layout.axes.len()) {
        return Err(LayoutError::AxisCount);
    }
    let pitch = i64::from(layout.gap) + 1;
    let mut seen = [false; 3];
    let mut total: u64 = 1;
    for axis in &layout.axes {
        if std::mem::replace(&mut seen[axis.dim as usize], true) {
            return Err(LayoutError::RepeatedDimension);
        }
        if axis.count == 0 {
            return Err(LayoutError::EmptyAxis);
        }
        total = total.saturating_mul(u64::from(axis.count));
    }
    if total > MAX_LAYOUT_VOXELS as u64 {
        return Err(LayoutError::TooManyVoxels);
    }
    for axis in &layout.axes {
        let extent = pitch * i64::from(axis.count - 1);
        let sign = if axis.positive { 1 } else { -1 };
        let end = i64::from(layout.anchor[axis.dim as usize]) + sign * extent;
        if i32::try_from(end).is_err() {
            return Err(LayoutError::OutOfRange);
        }
    }
    let pitch = pitch as i32;
    let total = total as u32;
    Ok((0..total)
        .map(|index| {
            let mut at = layout.anchor;
            let mut rest = index;
            for axis in &layout.axes {
                let step = (rest % axis.count) as i32;
                rest /= axis.count;
                let sign = if axis.positive { 1 } else { -1 };
                at[axis.dim as usize] += sign * pitch * step;
            }
            at
        })
        .collect())
}

/// The cells between two opposite corners, `axes` of the span's dimensions
/// wide: the longest first, a tie going to X, then Z, then Y. A dimension not
/// used stays at `a`'s coordinate, and the gap steps from `a` towards `b`, so
/// `b` itself is covered only where a whole step reaches it.
pub fn layout_between(
    a: [i32; 3],
    b: [i32; 3],
    axes: usize,
    gap: u32,
) -> Result<Vec<[i32; 3]>, LayoutError> {
    if !(1..=3).contains(&axes) {
        return Err(LayoutError::AxisCount);
    }
    let delta: [i64; 3] = std::array::from_fn(|i| i64::from(b[i]) - i64::from(a[i]));
    let mut order = [0usize, 2, 1];
    order.sort_by_key(|&i| std::cmp::Reverse(delta[i].unsigned_abs()));
    let pitch = u64::from(gap) + 1;
    let dims = [Dim::X, Dim::Y, Dim::Z];
    layout_cells(&Layout {
        anchor: a,
        axes: order[..axes]
            .iter()
            .map(|&i| LayoutAxis {
                dim: dims[i],
                positive: delta[i] >= 0,
                count: (delta[i].unsigned_abs() / pitch + 1).min(u64::from(u32::MAX)) as u32,
            })
            .collect(),
        gap,
    })
}

/// The dimension a span is flat on: the one `a` and `b` differ least along, a
/// tie going to Y, then Z, so a floor is flat in Y and a wall in X or Z.
pub fn span_normal(a: [i32; 3], b: [i32; 3]) -> usize {
    span_order(a, b)[2]
}

/// The dimensions from the longest difference to the shortest, a tie going to
/// X, then Z, then Y.
fn span_order(a: [i32; 3], b: [i32; 3]) -> [usize; 3] {
    let delta: [u64; 3] =
        std::array::from_fn(|i| (i64::from(b[i]) - i64::from(a[i])).unsigned_abs());
    let mut order = [0usize, 2, 1];
    order.sort_by_key(|&i| std::cmp::Reverse(delta[i]));
    order
}

/// A flat rectangle from `a` to `b` on the plane they span, given depth by `c`:
/// how far `c` lies from `a` along the dimension the rectangle is flat on, and
/// towards which side. With no `c` it is one Voxel deep. The gap steps from `a`
/// outwards on every dimension.
pub fn layout_span(
    a: [i32; 3],
    b: [i32; 3],
    c: Option<[i32; 3]>,
    gap: u32,
) -> Result<Vec<[i32; 3]>, LayoutError> {
    let order = span_order(a, b);
    let pitch = u64::from(gap) + 1;
    let dims = [Dim::X, Dim::Y, Dim::Z];
    let along = |from: [i32; 3], to: [i32; 3], i: usize| {
        let delta = i64::from(to[i]) - i64::from(from[i]);
        LayoutAxis {
            dim: dims[i],
            positive: delta >= 0,
            count: (delta.unsigned_abs() / pitch + 1).min(u64::from(u32::MAX)) as u32,
        }
    };
    layout_cells(&Layout {
        anchor: a,
        axes: vec![
            along(a, b, order[0]),
            along(a, b, order[1]),
            along(a, c.unwrap_or(a), order[2]),
        ],
        gap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(dim: Dim, positive: bool, count: u32) -> LayoutAxis {
        LayoutAxis {
            dim,
            positive,
            count,
        }
    }

    #[test]
    fn a_line_runs_from_the_anchor_along_its_axis() {
        let line = Layout {
            anchor: [2, 5, -1],
            axes: vec![axis(Dim::X, true, 3)],
            gap: 0,
        };
        assert_eq!(
            layout_cells(&line).unwrap(),
            [[2, 5, -1], [3, 5, -1], [4, 5, -1]]
        );
    }

    #[test]
    fn a_negative_axis_with_a_gap_steps_backwards_past_the_empty_voxels() {
        let line = Layout {
            anchor: [0, 0, 0],
            axes: vec![axis(Dim::Z, false, 3)],
            gap: 2,
        };
        assert_eq!(
            layout_cells(&line).unwrap(),
            [[0, 0, 0], [0, 0, -3], [0, 0, -6]]
        );
    }

    #[test]
    fn a_rectangle_fills_its_second_axis_after_the_first() {
        let rect = Layout {
            anchor: [0, 0, 0],
            axes: vec![axis(Dim::X, true, 2), axis(Dim::Z, true, 2)],
            gap: 1,
        };
        assert_eq!(
            layout_cells(&rect).unwrap(),
            [[0, 0, 0], [2, 0, 0], [0, 0, 2], [2, 0, 2]]
        );
    }

    #[test]
    fn a_box_with_mixed_directions_covers_every_combination_once() {
        let cuboid = Layout {
            anchor: [10, 0, 0],
            axes: vec![
                axis(Dim::X, false, 2),
                axis(Dim::Y, true, 2),
                axis(Dim::Z, true, 2),
            ],
            gap: 0,
        };
        assert_eq!(
            layout_cells(&cuboid).unwrap(),
            [
                [10, 0, 0],
                [9, 0, 0],
                [10, 1, 0],
                [9, 1, 0],
                [10, 0, 1],
                [9, 0, 1],
                [10, 1, 1],
                [9, 1, 1],
            ]
        );
    }

    fn line_of(count: u32) -> Layout {
        Layout {
            anchor: [0, 0, 0],
            axes: vec![axis(Dim::X, true, count)],
            gap: 0,
        }
    }

    #[test]
    fn exactly_the_cap_is_accepted_and_one_more_is_refused() {
        let at_cap = Layout {
            anchor: [0, 0, 0],
            axes: vec![axis(Dim::X, true, 16), axis(Dim::Z, true, 16)],
            gap: 0,
        };
        assert_eq!(layout_cells(&at_cap).unwrap().len(), MAX_LAYOUT_VOXELS);
        assert_eq!(
            layout_cells(&line_of(MAX_LAYOUT_VOXELS as u32 + 1)),
            Err(LayoutError::TooManyVoxels)
        );
    }

    #[test]
    fn the_cap_holds_however_the_counts_multiply() {
        let huge = Layout {
            anchor: [0, 0, 0],
            axes: vec![
                axis(Dim::X, true, u32::MAX),
                axis(Dim::Y, true, u32::MAX),
                axis(Dim::Z, true, u32::MAX),
            ],
            gap: 0,
        };
        assert_eq!(layout_cells(&huge), Err(LayoutError::TooManyVoxels));
    }

    #[test]
    fn a_malformed_axis_list_is_refused() {
        let mut layout = line_of(2);
        layout.axes.clear();
        assert_eq!(layout_cells(&layout), Err(LayoutError::AxisCount));
        layout.axes = vec![
            axis(Dim::X, true, 1),
            axis(Dim::Y, true, 1),
            axis(Dim::Z, true, 1),
            axis(Dim::X, true, 1),
        ];
        assert_eq!(layout_cells(&layout), Err(LayoutError::AxisCount));
        layout.axes = vec![axis(Dim::X, true, 2), axis(Dim::X, false, 2)];
        assert_eq!(layout_cells(&layout), Err(LayoutError::RepeatedDimension));
        assert_eq!(layout_cells(&line_of(0)), Err(LayoutError::EmptyAxis));
    }

    #[test]
    fn a_layout_reaching_past_the_coordinate_range_is_refused() {
        let mut layout = line_of(2);
        layout.anchor = [i32::MAX, 0, 0];
        assert_eq!(layout_cells(&layout), Err(LayoutError::OutOfRange));
        layout.gap = u32::MAX;
        layout.anchor = [0, 0, 0];
        assert_eq!(layout_cells(&layout), Err(LayoutError::OutOfRange));
    }

    #[test]
    fn a_line_between_corners_runs_the_longest_axis_and_keeps_the_rest_at_a() {
        assert_eq!(
            layout_between([1, 5, 2], [4, 6, 3], 1, 0).unwrap(),
            [[1, 5, 2], [2, 5, 2], [3, 5, 2], [4, 5, 2]]
        );
        assert_eq!(
            layout_between([0, 0, 0], [0, -2, 1], 1, 0).unwrap(),
            [[0, 0, 0], [0, -1, 0], [0, -2, 0]],
            "towards b, which is down here"
        );
    }

    #[test]
    fn a_rectangle_takes_the_two_longest_axes_and_is_flat_on_the_third() {
        let cells = layout_between([0, 3, 0], [4, 5, 2], 2, 0).unwrap();
        assert_eq!(cells.len(), 5 * 3);
        assert!(cells.iter().all(|c| c[1] == 3), "flat at a's height");
        assert!(cells.contains(&[4, 3, 2]) && cells.contains(&[0, 3, 0]));
    }

    #[test]
    fn a_box_fills_every_cell_from_a_to_b_inclusive() {
        let cells = layout_between([2, 2, 2], [0, 4, 3], 3, 0).unwrap();
        assert_eq!(cells.len(), 3 * 3 * 2);
        for x in 0..=2 {
            for y in 2..=4 {
                for z in 2..=3 {
                    assert!(cells.contains(&[x, y, z]), "{x} {y} {z}");
                }
            }
        }
    }

    #[test]
    fn ties_go_to_x_then_z_then_up() {
        let cells = layout_between([0, 0, 0], [2, 2, 2], 2, 0).unwrap();
        assert_eq!(cells.len(), 9);
        assert!(cells.iter().all(|c| c[1] == 0), "x and z, flat in y");
        let cells = layout_between([0, 0, 0], [0, 2, 2], 2, 0).unwrap();
        assert!(cells.iter().all(|c| c[0] == 0), "z and y when x is short");
    }

    #[test]
    fn the_gap_steps_from_a_towards_b_and_stops_short_of_b_when_it_does_not_land() {
        assert_eq!(
            layout_between([0, 0, 0], [4, 0, 0], 1, 1).unwrap(),
            [[0, 0, 0], [2, 0, 0], [4, 0, 0]]
        );
        assert_eq!(
            layout_between([0, 0, 0], [5, 0, 0], 1, 1).unwrap(),
            [[0, 0, 0], [2, 0, 0], [4, 0, 0]]
        );
    }

    #[test]
    fn one_point_is_one_cell_and_a_span_over_the_cap_is_refused() {
        assert_eq!(layout_between([3, 3, 3], [3, 3, 3], 3, 0).unwrap().len(), 1);
        assert_eq!(
            layout_between([0, 0, 0], [16, 0, 15], 2, 0),
            Err(LayoutError::TooManyVoxels),
            "17 by 16"
        );
        assert_eq!(
            layout_between([0, 0, 0], [15, 0, 15], 2, 0).unwrap().len(),
            MAX_LAYOUT_VOXELS
        );
        assert_eq!(
            layout_between([i32::MIN, 0, 0], [i32::MAX, 0, 0], 1, 0),
            Err(LayoutError::TooManyVoxels)
        );
        assert_eq!(
            layout_between([0; 3], [1; 3], 0, 0),
            Err(LayoutError::AxisCount)
        );
    }

    #[test]
    fn a_and_b_make_a_flat_rectangle_on_their_plane_and_c_gives_it_depth() {
        // A floor: B level with A, so it is flat in Y.
        let floor = layout_span([0, 5, 0], [3, 5, 2], None, 0).unwrap();
        assert_eq!(floor.len(), 4 * 3);
        assert!(floor.iter().all(|c| c[1] == 5));
        // C a few Voxels up gives the floor a height.
        let boxed = layout_span([0, 5, 0], [3, 5, 2], Some([9, 8, 9]), 0).unwrap();
        assert_eq!(
            boxed.len(),
            4 * 3 * 4,
            "c's X and Z are not used, only its Y"
        );
        assert!(boxed.contains(&[3, 8, 2]) && boxed.contains(&[0, 5, 0]));
    }

    #[test]
    fn c_below_or_level_with_the_plane_digs_down_or_stays_flat() {
        let down = layout_span([0, 5, 0], [1, 5, 1], Some([0, 3, 0]), 0).unwrap();
        assert_eq!(down.len(), 2 * 2 * 3);
        assert!(down.contains(&[1, 3, 1]));
        let flat = layout_span([0, 5, 0], [1, 5, 1], Some([7, 5, 7]), 0).unwrap();
        assert_eq!(flat.len(), 4, "level with A is no depth");
    }

    #[test]
    fn a_wall_is_flat_in_the_dimension_a_and_b_barely_differ_in() {
        // B differs in Y and X, not Z: a wall in the XY plane.
        assert_eq!(span_normal([0, 0, 0], [4, 3, 0]), 2);
        let wall = layout_span([0, 0, 0], [4, 3, 0], Some([0, 0, 2]), 0).unwrap();
        assert_eq!(wall.len(), 5 * 4 * 3);
        assert!(wall.contains(&[4, 3, 2]));
        // A tie between the shortest two goes to Y, so a floor stays a floor.
        assert_eq!(span_normal([0, 0, 0], [4, 1, 1]), 1);
    }

    #[test]
    fn the_span_gap_and_cap_follow_the_same_rules_as_any_layout() {
        assert_eq!(layout_span([0, 0, 0], [4, 0, 0], None, 1).unwrap().len(), 3);
        assert_eq!(
            layout_span([0, 0, 0], [15, 0, 15], Some([0, 2, 0]), 0),
            Err(LayoutError::TooManyVoxels),
            "16 by 16 by 3"
        );
        assert_eq!(
            layout_span([0, 0, 0], [15, 0, 15], None, 0).unwrap().len(),
            MAX_LAYOUT_VOXELS
        );
    }
}
