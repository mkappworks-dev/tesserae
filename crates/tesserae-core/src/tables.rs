//! Everything a native consumer asks about what a Voxel is.

use crate::light::MAX_LIGHT;
use crate::material::MaterialTable;
use crate::shape::{Orientation, ShapeTable};
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

/// Why [`Tables::vet_placed`] refused a Voxel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlaceFault {
    /// Placing air is a Break by another name.
    Air,
    UnknownMaterial,
    UnknownShape,
    /// An orientation byte of 24 or more.
    BadOrientation,
}

impl std::fmt::Display for PlaceFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PlaceFault::Air => "air is not placed, it is broken",
            PlaceFault::UnknownMaterial => "unknown material",
            PlaceFault::UnknownShape => "unknown shape",
            PlaceFault::BadOrientation => "invalid orientation",
        })
    }
}

impl Tables {
    /// Whether `voxel` may be placed: not air, and every id it names is in the tables. The one
    /// check a player's Place and a Mod's Command both pass.
    pub fn vet_placed(&self, voxel: Voxel) -> Result<(), PlaceFault> {
        if voxel.is_air() {
            return Err(PlaceFault::Air);
        }
        if !self.materials.contains(voxel.material()) {
            return Err(PlaceFault::UnknownMaterial);
        }
        if !self.shapes.contains(voxel.shape()) {
            return Err(PlaceFault::UnknownShape);
        }
        if Orientation::from_bits(voxel.orientation()).is_none() {
            return Err(PlaceFault::BadOrientation);
        }
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::STONE;

    #[test]
    fn vet_placed_names_the_first_thing_wrong() {
        let tables = Tables::builtin();
        let materials = tables.materials.len() as u16;
        let shapes = tables.shapes.len() as u8;
        assert_eq!(tables.vet_placed(Voxel::cube(STONE)), Ok(()));
        assert_eq!(tables.vet_placed(Voxel::AIR), Err(PlaceFault::Air));
        assert_eq!(
            tables.vet_placed(Voxel::new(materials, 0, 0)),
            Err(PlaceFault::UnknownMaterial)
        );
        assert_eq!(
            tables.vet_placed(Voxel::new(STONE, shapes, 0)),
            Err(PlaceFault::UnknownShape)
        );
        assert_eq!(
            tables.vet_placed(Voxel::new(STONE, 0, 24)),
            Err(PlaceFault::BadOrientation)
        );
        assert_eq!(tables.vet_placed(Voxel::new(STONE, 0, 23)), Ok(()));
    }
}
