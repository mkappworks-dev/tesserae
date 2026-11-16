//! Templates: blocks of Voxels on a grid in metres, which a World samples at
//! its own Voxel size and turns in quarter turns. ADR-0029.

use crate::shape::Orientation;
use crate::voxel::{MaterialId, Voxel};
use crate::worldgen::GroundParams;

/// The longest side of a Template, in metres.
pub const MAX_TEMPLATE_SIDE_M: f32 = 64.0;
pub const MAX_TEMPLATE_CELLS: usize = 65_536;
/// Templates in a World, from every Mod together.
pub const MAX_TEMPLATES: usize = 64;
pub const DEFAULT_LEVEL_TOLERANCE_M: f32 = 1.0;
pub const MAX_LEVEL_TOLERANCE_M: f32 = 16.0;

/// Slack on metre comparisons, so a side of exactly 64 m or a Voxel centre on a
/// cell edge does not flip with float error.
const GUARD: f32 = 1e-3;

/// One cell of a Template.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cell {
    /// Never written.
    Keep,
    /// Written as air, so it carves.
    Air,
    Voxel(Voxel),
    /// The ground where it stands: its column's Biome's surface Material at
    /// and above the Ground layer, its soil below.
    Ground,
}

/// Why a Template's grid is refused.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TemplateError {
    CellSize,
    NoCells,
    /// A layer with a different number of rows than the first, or a row of a
    /// different length.
    Ragged {
        layer: usize,
        row: usize,
    },
    TooLong {
        axis: char,
        metres: f32,
    },
    TooManyCells(usize),
    NoGroundLayer {
        layer: usize,
        layers: usize,
    },
    Tolerance,
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::CellSize => f.write_str("cell-m must be above 0 metres"),
            TemplateError::NoCells => f.write_str("has no cells"),
            TemplateError::Ragged { layer, row } => write!(
                f,
                "layer {layer}, row {row}: every layer needs the same rows and every row the \
                 same length"
            ),
            TemplateError::TooLong { axis, metres } => write!(
                f,
                "is {metres} m along {axis}; a side is at most {MAX_TEMPLATE_SIDE_M} m"
            ),
            TemplateError::TooManyCells(count) => write!(
                f,
                "has {count} cells; a Template has at most {MAX_TEMPLATE_CELLS}"
            ),
            TemplateError::NoGroundLayer { layer, layers } => {
                write!(f, "ground-layer {layer} is not one of its {layers} layers")
            }
            TemplateError::Tolerance => write!(
                f,
                "level-tolerance-m must be from 0 to {MAX_LEVEL_TOLERANCE_M} metres"
            ),
        }
    }
}

impl std::error::Error for TemplateError {}

/// A quarter-turn count about the World's +Y, the way
/// [`Orientation::yawed`] turns: the Template's +Z edge comes to face +X.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Turn(u8);

impl Turn {
    pub const NONE: Turn = Turn(0);

    pub const fn new(quarters: u8) -> Self {
        Turn(quarters % 4)
    }

    pub const fn quarters(self) -> u8 {
        self.0
    }

    pub const fn next(self) -> Self {
        Turn::new(self.0 + 1)
    }

    pub fn all() -> impl Iterator<Item = Turn> {
        (0..4).map(Turn)
    }
}

/// A named block of cells, `x` along +X, `y` up, `z` along +Z. Built only
/// through [`Template::new`], so its grid is always within the limits.
#[derive(Clone, PartialEq, Debug)]
pub struct Template {
    /// `<mod id>:<name>`.
    pub name: String,
    cell_m: f32,
    size: [usize; 3],
    cells: Vec<Cell>,
    ground_layer: usize,
    foundation: Option<MaterialId>,
    level_tolerance_m: f32,
}

impl Template {
    /// `layers` from the bottom up, each rows along +Z of cells along +X.
    pub fn new(
        name: String,
        cell_m: f32,
        layers: Vec<Vec<Vec<Cell>>>,
        ground_layer: usize,
        foundation: Option<MaterialId>,
        level_tolerance_m: f32,
    ) -> Result<Self, TemplateError> {
        if !(cell_m.is_finite() && cell_m > 0.0) {
            return Err(TemplateError::CellSize);
        }
        if !(0.0..=MAX_LEVEL_TOLERANCE_M).contains(&level_tolerance_m) {
            return Err(TemplateError::Tolerance);
        }
        let rows = layers.first().map_or(0, Vec::len);
        let width = layers.first().and_then(|l| l.first()).map_or(0, Vec::len);
        if width == 0 {
            return Err(TemplateError::NoCells);
        }
        for (y, layer) in layers.iter().enumerate() {
            if layer.len() != rows {
                return Err(TemplateError::Ragged {
                    layer: y,
                    row: layer.len().min(rows),
                });
            }
            if let Some(z) = layer.iter().position(|row| row.len() != width) {
                return Err(TemplateError::Ragged { layer: y, row: z });
            }
        }
        let size = [width, layers.len(), rows];
        for (axis, n) in ['x', 'y', 'z'].into_iter().zip(size) {
            let metres = n as f32 * cell_m;
            if metres > MAX_TEMPLATE_SIDE_M + GUARD {
                return Err(TemplateError::TooLong { axis, metres });
            }
        }
        let count = size.iter().product();
        if count > MAX_TEMPLATE_CELLS {
            return Err(TemplateError::TooManyCells(count));
        }
        if ground_layer >= size[1] {
            return Err(TemplateError::NoGroundLayer {
                layer: ground_layer,
                layers: size[1],
            });
        }
        Ok(Template {
            name,
            cell_m,
            size,
            cells: layers.into_iter().flatten().flatten().collect(),
            ground_layer,
            foundation,
            level_tolerance_m,
        })
    }

    pub fn cell_m(&self) -> f32 {
        self.cell_m
    }

    /// Cells along x, y and z.
    pub fn size(&self) -> [usize; 3] {
        self.size
    }

    pub fn size_m(&self) -> [f32; 3] {
        self.size.map(|n| n as f32 * self.cell_m)
    }

    /// The cell at `[x, y, z]`, as authored.
    pub fn cell(&self, [x, y, z]: [usize; 3]) -> Cell {
        self.cells[(y * self.size[2] + z) * self.size[0] + x]
    }

    pub fn ground_layer(&self) -> usize {
        self.ground_layer
    }

    pub fn foundation(&self) -> Option<MaterialId> {
        self.foundation
    }

    pub fn level_tolerance_m(&self) -> f32 {
        self.level_tolerance_m
    }

    /// The cell `[x, z]` of its door: the first cell `is_door` takes, rows in
    /// order, in the lowest layer from the Ground layer up that has one. A door
    /// stands on a floor, so it may be a layer above the Ground layer.
    pub fn door_cell(&self, is_door: impl Fn(MaterialId) -> bool) -> Option<[u16; 2]> {
        let [w, h, d] = self.size;
        (self.ground_layer..h).find_map(|y| {
            (0..d).find_map(|z| {
                (0..w).find_map(|x| match self.cell([x, y, z]) {
                    Cell::Voxel(v) if is_door(v.material()) => Some([x as u16, z as u16]),
                    _ => None,
                })
            })
        })
    }

    /// The Template as a World of `voxel_size` holds it, turned by `turn`.
    pub fn sample(&self, voxel_size: f32, turn: Turn) -> Sampled<'_> {
        let along = |axis: usize| -> Vec<u16> {
            let extent = self.size[axis] as f32 * self.cell_m / voxel_size;
            // The Voxels whose centre falls inside the side.
            let voxels = ((extent - 0.5 - GUARD).ceil() as usize).max(1);
            (0..voxels)
                .map(|i| {
                    let cell = ((i as f32 + 0.5) * voxel_size / self.cell_m + GUARD).floor();
                    (cell as usize).min(self.size[axis] - 1) as u16
                })
                .collect()
        };
        let cells = [along(0), along(1), along(2)];
        let ground = cells[1]
            .iter()
            .position(|&c| c as usize >= self.ground_layer)
            .unwrap_or(cells[1].len()) as i32;
        let mut orientations = [0u8; 24];
        for o in Orientation::all() {
            orientations[o.bits() as usize] = o.yawed(turn.quarters()).bits();
        }
        Sampled {
            template: self,
            turn,
            cells,
            ground,
            orientations,
        }
    }
}

/// A [`Template`] at one Voxel size and turn. Coordinates are Voxels from the
/// turned footprint's lowest corner.
#[derive(Clone, Debug)]
pub struct Sampled<'a> {
    template: &'a Template,
    turn: Turn,
    /// For each axis of the unturned Template, each Voxel's cell.
    cells: [Vec<u16>; 3],
    ground: i32,
    /// Each orientation byte after the turn.
    orientations: [u8; 24],
}

impl Sampled<'_> {
    pub fn template(&self) -> &Template {
        self.template
    }

    pub fn turn(&self) -> Turn {
        self.turn
    }

    /// Voxels along the World's x, y and z, after the turn.
    pub fn size(&self) -> [i32; 3] {
        let [x, y, z] = self.cells.each_ref().map(|c| c.len() as i32);
        if self.turn.quarters().is_multiple_of(2) {
            [x, y, z]
        } else {
            [z, y, x]
        }
    }

    /// The lowest Voxel layer of the Ground layer; the layers below it are
    /// underground.
    pub fn ground(&self) -> i32 {
        self.ground
    }

    /// What a [`Cell::Ground`] in Voxel layer `y` is where the column's
    /// ground is `ground`.
    pub fn ground_voxel(&self, y: i32, ground: GroundParams) -> Voxel {
        Voxel::cube(if y < self.ground {
            ground.soil
        } else {
            ground.surface
        })
    }

    /// The cell at `at`, each in `0..size()`, with its Orientation turned.
    pub fn cell(&self, [x, y, z]: [i32; 3]) -> Cell {
        let [w, _, d] = self.cells.each_ref().map(|c| c.len() as i32);
        // Undo the turn: a quarter takes the Template's (x, z) to (z, w - 1 - x).
        let (tx, tz) = match self.turn.quarters() {
            0 => (x, z),
            1 => (w - 1 - z, x),
            2 => (w - 1 - x, d - 1 - z),
            _ => (z, d - 1 - x),
        };
        let cell = self.template.cell([
            self.cells[0][tx as usize] as usize,
            self.cells[1][y as usize] as usize,
            self.cells[2][tz as usize] as usize,
        ]);
        match cell {
            Cell::Voxel(v) => Cell::Voxel(Voxel::new(
                v.material(),
                v.shape(),
                self.orientations
                    .get(v.orientation() as usize)
                    .copied()
                    .unwrap_or(self.orientations[0]),
            )),
            other => other,
        }
    }
}
