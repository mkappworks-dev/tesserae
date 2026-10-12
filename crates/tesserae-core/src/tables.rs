//! Everything a native consumer asks about what a Voxel is.

use crate::material::MaterialTable;
use crate::shape::ShapeTable;

/// A Voxel's kind, as the pair of tables that describe it: the [`ShapeTable`]
/// for geometry and the [`MaterialTable`] for the rest.
///
/// One value, so a mod loader fills it once, the Server holds it once, and a
/// function that needs both takes one reference and not two. Functions that read
/// only geometry (collision) still take a `&ShapeTable`.
#[derive(Clone, Debug, Default)]
pub struct Tables {
    pub shapes: ShapeTable,
    pub materials: MaterialTable,
}

impl Tables {
    pub fn builtin() -> Self {
        Self::default()
    }
}
