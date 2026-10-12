//! Everything a native consumer asks about what a Voxel is.

use crate::light::MAX_LIGHT;
use crate::material::MaterialTable;
use crate::shape::ShapeTable;
use crate::voxel::Voxel;

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

    /// What the light solvers ask of a Voxel: the faces light may not enter
    /// through (see [`ShapeTable::light_mask`]), and the levels it removes from
    /// light that does.
    ///
    /// An opaque Material (dimming 15) is blocked by its geometry and removes
    /// nothing: a stair stores the light of its own notch undimmed. Anything
    /// clearer is open on every face whatever its Shape, and removes its
    /// Material's dimming once per entry. So the two never both apply, and a
    /// glass stair casts no stair-shaped shadow.
    #[inline]
    pub fn light_params(&self, voxel: Voxel) -> (u8, u8) {
        if voxel.is_air() {
            return (0, 0);
        }
        match self.materials.dimming(voxel) {
            MAX_LIGHT => (self.shapes.light_mask(voxel), 0),
            dimming => (0, dimming),
        }
    }

    /// The first half of [`Tables::light_params`], for a caller that holds a
    /// neighbour's Voxel only to ask whether light may leave it.
    #[inline]
    pub fn light_mask(&self, voxel: Voxel) -> u8 {
        self.light_params(voxel).0
    }

    /// [`ShapeTable::joint_id`] for a Voxel that can seal a plane at all: an
    /// opaque Material, whose geometry decides what light does. A clear one
    /// seals nothing alone or in a pair, so it has no joint id.
    #[inline]
    pub fn joint_id(&self, voxel: Voxel) -> u16 {
        if voxel.is_air() || self.materials.dimming(voxel) != MAX_LIGHT {
            return 0;
        }
        self.shapes.joint_id(voxel)
    }
}
