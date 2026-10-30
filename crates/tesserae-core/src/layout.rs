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
}
