//! The atomic unit of world volume.

/// What a Voxel is made of.
pub type MaterialId = u16;

/// A Voxel's geometry.
pub type ShapeId = u8;

/// The absence of Material. Air is a Voxel, not a null.
pub const AIR: MaterialId = 0;

/// The only Shape the greedy mesher can merge. Everything else needs per-Voxel
/// geometry in the mesher's second pass.
pub const SHAPE_FULL_CUBE: ShapeId = 0;

/// Packed `material:16 | shape:8 | orientation:8`.
///
/// Exactly a `u32`, so a palette entry costs one word and two Voxels compare
/// without touching the palette. Shape and orientation are reserved now because
/// widening this later would be a save-format migration: a Voxel is
/// `(Material, Shape, Orientation)` and never a bare Material.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Voxel(u32);

impl Voxel {
    pub const AIR: Voxel = Voxel(0);

    #[inline]
    pub const fn new(material: MaterialId, shape: ShapeId, orientation: u8) -> Self {
        Voxel((material as u32) << 16 | (shape as u32) << 8 | orientation as u32)
    }

    #[inline]
    pub const fn cube(material: MaterialId) -> Self {
        Self::new(material, SHAPE_FULL_CUBE, 0)
    }

    #[inline]
    pub const fn material(self) -> MaterialId {
        (self.0 >> 16) as MaterialId
    }

    #[inline]
    pub const fn shape(self) -> ShapeId {
        (self.0 >> 8) as ShapeId
    }

    #[inline]
    pub const fn orientation(self) -> u8 {
        self.0 as u8
    }

    #[inline]
    pub const fn is_air(self) -> bool {
        self.material() == AIR
    }

    /// Whether this Voxel both emits greedy-mergeable faces and fully occludes
    /// its neighbours'. A shaped Voxel does neither until the second mesher pass
    /// and per-Shape opacity exist, so it is conservatively transparent.
    #[inline]
    pub const fn is_full_cube(self) -> bool {
        !self.is_air() && self.shape() == SHAPE_FULL_CUBE
    }
}

impl Default for Voxel {
    fn default() -> Self {
        Voxel::AIR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_round_trip_independently() {
        let v = Voxel::new(0xBEEF, 0x12, 0x34);
        assert_eq!(v.material(), 0xBEEF);
        assert_eq!(v.shape(), 0x12);
        assert_eq!(v.orientation(), 0x34);
    }

    #[test]
    fn air_is_material_zero_whatever_the_shape() {
        assert!(Voxel::AIR.is_air());
        assert!(Voxel::new(AIR, 7, 3).is_air());
        assert!(!Voxel::cube(1).is_air());
    }

    #[test]
    fn only_unshaped_solid_voxels_are_full_cubes() {
        assert!(Voxel::cube(1).is_full_cube());
        assert!(!Voxel::AIR.is_full_cube());
        assert!(!Voxel::new(1, 5, 0).is_full_cube());
    }

    #[test]
    fn voxel_is_word_sized() {
        assert_eq!(size_of::<Voxel>(), 4);
    }
}
