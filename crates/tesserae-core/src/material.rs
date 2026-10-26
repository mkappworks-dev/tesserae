//! What a Voxel is made of, as table data.
//!
//! A [`MaterialTable`] holds the properties the engine asks of a Material -
//! what it emits, how it dims light, what faces it hides, which pass draws it -
//! in rows indexed by [`MaterialId`], as [`crate::ShapeTable`] does for Shapes.
//! Geometry is the Shape's and appearance is a Look pack's; this is what is
//! neither. `docs/adr/0009-materials-are-table-data.md`.

use crate::light::MAX_LIGHT;
use crate::voxel::{AIR, MaterialId, Voxel};

pub const FIRST_MOD_ID: MaterialId = 8;
pub const RETIRED_IDS: std::ops::Range<MaterialId> = 1..FIRST_MOD_ID;

/// Whether a full cube of this Material hides the faces around it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Occlusion {
    /// Hides every neighbour's face. Stone.
    Opaque,
    /// Hides the faces of the same Material only, so a wall of glass is a skin
    /// and not a honeycomb while glass beside stone keeps stone's face.
    SameMaterial,
    /// Hides nothing, not even itself: every face of a leaf is drawn.
    Never,
}

/// Which mesh a Material's faces go into. Semantic, so it lives here and not
/// in a Look pack: the mesher decides it and knows nothing of Bevy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderClass {
    Opaque,
    Blend,
    /// Opaque where drawn, with holes: written to depth and casting a shadow
    /// with its holes, but on a pass of its own, because a pass that may
    /// discard costs every fragment on it. Leaves.
    Cutout,
}

/// One row of the [`MaterialTable`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MaterialDef {
    /// Block light the Voxel emits, `0..=MAX_LIGHT`.
    pub emission: u8,
    /// Levels removed from light entering the Voxel, on top of the one every
    /// step costs, `0..=MAX_LIGHT`. 15 is opaque. A light level counts
    /// attenuation steps and is not a distance, so this does not scale with
    /// Voxel size.
    pub dimming: u8,
    pub occlusion: Occlusion,
    pub render: RenderClass,
    /// Whether a body collides with it and the edit ray stops on it. A fluid is
    /// not: a body sinks through it and a ray goes through it to the bed.
    pub solid: bool,
}

impl MaterialDef {
    /// What stone is, and what an id the table does not know answers as.
    pub const OPAQUE: MaterialDef = MaterialDef {
        emission: 0,
        dimming: MAX_LIGHT,
        occlusion: Occlusion::Opaque,
        render: RenderClass::Opaque,
        solid: true,
    };

    const AIR: MaterialDef = MaterialDef {
        emission: 0,
        dimming: 0,
        occlusion: Occlusion::Never,
        render: RenderClass::Opaque,
        solid: false,
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialError {
    /// `emission` or `dimming` above [`MAX_LIGHT`].
    LightOutOfRange,
    /// `Occlusion::Opaque` with a dimming under 15 would be a light leak by
    /// construction, and with `RenderClass::Blend` a draw call spent on a face
    /// that hides everything behind it.
    OpaqueNotOpaque,
    /// Every id a `u16` can hold is taken.
    TableFull,
}

impl std::fmt::Display for MaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MaterialError::LightOutOfRange => "emission and dimming are at most 15",
            MaterialError::OpaqueNotOpaque => {
                "an Opaque Material needs dimming 15 and the opaque render class"
            }
            MaterialError::TableFull => "all 65536 MaterialIds are taken",
        })
    }
}

impl std::error::Error for MaterialError {}

/// Every registered Material.
///
/// Filled by one batched `register` per Material at load and read natively
/// afterwards, so a Mod never makes a per-Voxel call across the WASM boundary.
/// `docs/adr/0001-wasm-mod-boundary.md`.
///
/// Rows are dense and sized to the ids handed out, not to `u16::MAX`: ids are
/// handed out in order, so a table of N Materials is 5N bytes and not 64 KiB of
/// zeroes. [`RETIRED_IDS`] keeps a row, the stand-in, so the ids after it stay put.
#[derive(Clone, Debug)]
pub struct MaterialTable {
    defs: Vec<MaterialDef>,
}

impl Default for MaterialTable {
    fn default() -> Self {
        Self::builtin()
    }
}

impl MaterialTable {
    /// The Materials every World has, under permanent ids: a Mod's ids start
    /// after them, and a saved World refers to them by number.
    pub fn builtin() -> Self {
        let mut defs = vec![MaterialDef::OPAQUE; FIRST_MOD_ID as usize];
        defs[AIR as usize] = MaterialDef::AIR;
        MaterialTable { defs }
    }

    pub fn register(&mut self, def: MaterialDef) -> Result<MaterialId, MaterialError> {
        if def.emission > MAX_LIGHT || def.dimming > MAX_LIGHT {
            return Err(MaterialError::LightOutOfRange);
        }
        if def.occlusion == Occlusion::Opaque
            && (def.dimming != MAX_LIGHT || def.render != RenderClass::Opaque)
        {
            return Err(MaterialError::OpaqueNotOpaque);
        }
        let id = MaterialId::try_from(self.defs.len()).map_err(|_| MaterialError::TableFull)?;
        self.defs.push(def);
        Ok(id)
    }

    /// Ids handed out so far, [`RETIRED_IDS`] included: the next registered id.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    pub fn contains(&self, material: MaterialId) -> bool {
        (material as usize) < self.defs.len() && !RETIRED_IDS.contains(&material)
    }

    /// An unregistered id answers as [`MaterialDef::OPAQUE`]: a World whose Mod
    /// is missing keeps a solid, drawn stand-in rather than holes. The opposite
    /// of an unknown Shape, which is invisible, because a Shape's geometry
    /// cannot be guessed and a Material's worst case is safe.
    #[inline]
    pub fn def(&self, material: MaterialId) -> &MaterialDef {
        self.defs
            .get(material as usize)
            .unwrap_or(&MaterialDef::OPAQUE)
    }

    /// How much block light `voxel` emits.
    #[inline]
    pub fn emission(&self, voxel: Voxel) -> u8 {
        self.def(voxel.material()).emission
    }

    #[inline]
    pub fn dimming(&self, voxel: Voxel) -> u8 {
        self.def(voxel.material()).dimming
    }

    #[inline]
    pub fn occlusion(&self, voxel: Voxel) -> Occlusion {
        self.def(voxel.material()).occlusion
    }

    #[inline]
    pub fn render(&self, voxel: Voxel) -> RenderClass {
        self.def(voxel.material()).render
    }

    /// Air is never solid, whatever Material id it carries.
    #[inline]
    pub fn solid(&self, voxel: Voxel) -> bool {
        !voxel.is_air() && self.def(voxel.material()).solid
    }

    /// Hides every neighbouring face and stops all light, whatever the Shape.
    #[inline]
    pub fn is_opaque(&self, voxel: Voxel) -> bool {
        self.def(voxel.material()).dimming == MAX_LIGHT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(table: &MaterialTable, id: MaterialId) -> MaterialDef {
        *table.def(id)
    }

    #[test]
    fn air_is_the_only_builtin_and_retired_ids_are_solid_stand_ins() {
        let t = MaterialTable::builtin();
        assert_eq!(t.len(), FIRST_MOD_ID as usize);
        assert!(t.contains(AIR));
        assert_eq!(def(&t, AIR), MaterialDef::AIR);
        for id in RETIRED_IDS {
            assert!(!t.contains(id));
            assert_eq!(def(&t, id), MaterialDef::OPAQUE);
            assert!(t.solid(Voxel::cube(id)));
            assert!(t.is_opaque(Voxel::cube(id)));
        }
    }

    #[test]
    fn a_mods_first_material_still_gets_eight() {
        let mut t = MaterialTable::builtin();
        assert_eq!(t.register(crate::testgen::WATER_MATERIAL), Ok(8));
        assert!(!t.contains(FIRST_MOD_ID.saturating_sub(1)) && t.contains(8));
        assert_eq!(def(&t, FIRST_MOD_ID.saturating_sub(1)), MaterialDef::OPAQUE);
    }

    #[test]
    fn no_builtin_emits() {
        let t = MaterialTable::builtin();
        for id in 0..FIRST_MOD_ID {
            assert_eq!(t.emission(Voxel::cube(id)), 0);
        }
    }

    #[test]
    fn an_id_the_table_does_not_know_is_a_solid_stand_in() {
        let t = MaterialTable::builtin();
        let missing = Voxel::cube(999);
        assert!(!t.contains(999));
        assert!(t.is_opaque(missing));
        assert_eq!(t.emission(missing), 0);
        assert_eq!(t.occlusion(missing), Occlusion::Opaque);
        assert_eq!(t.render(missing), RenderClass::Opaque);
        assert_eq!(t.dimming(Voxel::cube(u16::MAX)), MAX_LIGHT);
    }

    #[test]
    fn only_a_dimming_of_fifteen_is_opaque_whatever_the_shape() {
        let t = &crate::testgen::fixtures().tables.materials;
        assert!(t.is_opaque(Voxel::cube(crate::testgen::fixtures().ground.rock)));
        assert!(t.is_opaque(Voxel::new(crate::testgen::fixtures().ground.rock, 2, 5)));
        for id in [
            crate::testgen::fixtures().glass,
            crate::testgen::fixtures().leaves,
        ] {
            assert!(!t.is_opaque(Voxel::cube(id)), "material {id}");
        }
    }

    #[test]
    fn registered_materials_take_the_next_ids_in_order() {
        let mut t = MaterialTable::builtin();
        let amber = MaterialDef {
            emission: 8,
            dimming: 1,
            occlusion: Occlusion::SameMaterial,
            render: RenderClass::Blend,
            solid: true,
        };
        assert_eq!(t.register(amber), Ok(8));
        assert_eq!(t.register(MaterialDef::OPAQUE), Ok(9));
        assert_eq!(t.emission(Voxel::cube(8)), 8);
        assert!(t.contains(9) && !t.contains(10));
    }

    #[test]
    fn register_rejects_what_cannot_be_a_material() {
        let mut t = MaterialTable::builtin();
        let ok = crate::testgen::WATER_MATERIAL;
        for bad in [
            MaterialDef {
                emission: MAX_LIGHT + 1,
                ..ok
            },
            MaterialDef {
                dimming: MAX_LIGHT + 1,
                ..ok
            },
        ] {
            assert_eq!(t.register(bad), Err(MaterialError::LightOutOfRange));
        }
        for bad in [
            MaterialDef {
                dimming: 14,
                ..MaterialDef::OPAQUE
            },
            MaterialDef {
                render: RenderClass::Blend,
                ..MaterialDef::OPAQUE
            },
        ] {
            assert_eq!(t.register(bad), Err(MaterialError::OpaqueNotOpaque));
        }
        assert_eq!(t.len(), 8, "a rejected row takes no id");
    }

    #[test]
    fn the_table_is_full_at_sixty_five_thousand_five_hundred_thirty_six_rows() {
        let mut t = MaterialTable::builtin();
        while t.len() < u16::MAX as usize + 1 {
            t.register(MaterialDef::OPAQUE).unwrap();
        }
        assert_eq!(
            t.register(MaterialDef::OPAQUE),
            Err(MaterialError::TableFull)
        );
        assert_eq!(t.len(), u16::MAX as usize + 1);
    }

    #[test]
    fn a_row_is_five_bytes() {
        assert_eq!(size_of::<MaterialDef>(), 5);
    }
}
