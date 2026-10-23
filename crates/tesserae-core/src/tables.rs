//! Everything a native consumer asks about what a Voxel is.

use crate::fluid::{FluidDef, FluidError, FluidTable};
use crate::light::MAX_LIGHT;
use crate::material::MaterialTable;
use crate::occupants::{Fluid, Level, Occupants};
use crate::shape::{Orientation, ShapeTable};
use crate::voxel::{MaterialId, Voxel};

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
    pub fluids: FluidTable,
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
    /// A fluid Material where a solid goes: a fluid is an occupant, not a Shape.
    NotSolid,
    /// A fluid whose Material has no fluid definition.
    NotAFluid,
    /// Fluid in a solid that leaves no void for it: a cube, or a Shape whose
    /// empty space splits into more voids than fluid state can tell apart.
    FluidExcluded,
    /// A surface above [`Fluid::FULL_SURFACE`], flow that is dry everywhere or
    /// wet in a void the solid does not have, or a source that is not full or
    /// carries flow state.
    BadSurface,
    /// Burning with nothing solid to burn.
    NoFuel,
}

impl std::fmt::Display for PlaceFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PlaceFault::Air => "air is not placed, it is broken",
            PlaceFault::UnknownMaterial => "unknown material",
            PlaceFault::UnknownShape => "unknown shape",
            PlaceFault::BadOrientation => "invalid orientation",
            PlaceFault::NotSolid => "a fluid is not placed as a solid",
            PlaceFault::NotAFluid => "not a fluid",
            PlaceFault::FluidExcluded => "the solid leaves no room for fluid",
            PlaceFault::BadSurface => "invalid fluid surface",
            PlaceFault::NoFuel => "nothing solid to burn",
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

    /// Whether `occupants` may be stored: a vetted solid or air, a fluid the
    /// tables know with room for it, and fire only where there is fuel. The
    /// one check every writer of World state passes.
    pub fn vet_occupants(&self, occupants: Occupants) -> Result<(), PlaceFault> {
        if !occupants.solid.is_air() {
            self.vet_placed(occupants.solid)?;
            if self.fluids.contains(occupants.solid.material()) {
                return Err(PlaceFault::NotSolid);
            }
        }
        if let Some(fluid) = occupants.fluid {
            if !self.fluids.contains(fluid.material) {
                return Err(PlaceFault::NotAFluid);
            }
            let slots = self.shapes.void_slots(occupants.solid);
            if slots == 0 {
                return Err(PlaceFault::FluidExcluded);
            }
            let canonical = if fluid.source {
                fluid == Fluid::source(fluid.material)
            } else {
                let levels = fluid.levels();
                levels.iter().enumerate().all(|(i, l)| {
                    if l.is_wet() {
                        i < slots && l.surface <= Fluid::FULL_SURFACE
                    } else {
                        *l == Level::DRY
                    }
                }) && levels.iter().any(|l| l.is_wet())
                    // Empty space has one void, which flow fills.
                    && (!occupants.solid.is_air() || levels[0].is_wet())
            };
            if !canonical {
                return Err(PlaceFault::BadSurface);
            }
        }
        if occupants.burning.is_some() && occupants.solid.is_air() {
            return Err(PlaceFault::NoFuel);
        }
        Ok(())
    }

    /// Registers how a Material flows. The Material comes first, from the same
    /// batched Declaration, and must not be solid.
    pub fn register_fluid(
        &mut self,
        material: MaterialId,
        def: FluidDef,
    ) -> Result<(), FluidError> {
        if !self.materials.contains(material) {
            return Err(FluidError::NotAMaterial);
        }
        if self.materials.solid(Voxel::cube(material)) {
            return Err(FluidError::Solid);
        }
        self.fluids.register(material, def)
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

    /// [`Tables::light_params`] of `voxel` with `fluid` standing in its voids.
    ///
    /// A Voxel holds one light value, so this is an approximation both solvers
    /// share: the solid's seals stay, and the dimming is the larger of the
    /// solid's and the fluid's rather than charged twice, whichever void the
    /// fluid is in. Separate voids of one Voxel share its brightness. Fluid
    /// and heat passage never read this as geometry; they read voids.
    #[inline]
    pub fn wet_light_params(&self, voxel: Voxel, fluid: Option<MaterialId>) -> (u8, u8) {
        let Some(fluid) = fluid else {
            return self.light_params(voxel);
        };
        let cube = Voxel::cube(fluid);
        if voxel.is_air() {
            return self.light_params(cube);
        }
        let (mask, dim) = self.light_params(voxel);
        (mask, dim.max(self.materials.dimming(cube)))
    }

    /// Emission under the same approximation: the brighter of solid and fluid.
    #[inline]
    pub fn wet_emission(&self, voxel: Voxel, fluid: Option<MaterialId>) -> u8 {
        let own = self.materials.emission(voxel);
        fluid.map_or(own, |f| own.max(self.materials.emission(Voxel::cube(f))))
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
    use crate::fluid::{FluidDef, FluidError};
    use crate::material::{MaterialDef, Occlusion, RenderClass, STONE, WATER};
    use crate::occupants::{Burning, Fluid, Occupants};

    #[test]
    fn a_fluid_is_registered_over_a_non_solid_material_only() {
        let mut tables = Tables::builtin();
        let decor = tables
            .materials
            .register(MaterialDef {
                solid: false,
                emission: 0,
                dimming: 0,
                occlusion: Occlusion::Never,
                render: RenderClass::Blend,
            })
            .unwrap();
        assert_eq!(
            tables.register_fluid(999, FluidDef::WATER),
            Err(FluidError::NotAMaterial)
        );
        assert_eq!(
            tables.register_fluid(STONE, FluidDef::WATER),
            Err(FluidError::Solid)
        );
        assert_eq!(tables.register_fluid(decor, FluidDef::WATER), Ok(()));
        assert!(tables.fluids.contains(decor));
    }

    /// What the shared Edit path refuses of a reading before it is stored.
    #[test]
    fn vet_occupants_names_the_first_thing_wrong() {
        let tables = Tables::builtin();
        let water = Fluid::source(WATER);
        let ok = |solid, fluid, burning| Occupants {
            solid,
            fluid,
            burning,
        };
        let stair = Voxel::new(STONE, crate::SHAPE_STAIR, 0);
        assert_eq!(
            tables.vet_occupants(ok(Voxel::AIR, Some(water), None)),
            Ok(())
        );
        assert_eq!(
            tables.vet_occupants(ok(stair, Some(water), Some(Burning { fuel: 1 }))),
            Ok(())
        );
        assert_eq!(tables.vet_occupants(ok(Voxel::AIR, None, None)), Ok(()));
        assert_eq!(
            tables.vet_occupants(ok(Voxel::new(STONE, 0, 24), None, None)),
            Err(PlaceFault::BadOrientation)
        );
        assert_eq!(
            tables.vet_occupants(ok(Voxel::cube(WATER), None, None)),
            Err(PlaceFault::NotSolid)
        );
        assert_eq!(
            tables.vet_occupants(ok(Voxel::AIR, Some(Fluid::source(STONE)), None)),
            Err(PlaceFault::NotAFluid)
        );
        assert_eq!(
            tables.vet_occupants(ok(Voxel::cube(STONE), Some(water), None)),
            Err(PlaceFault::FluidExcluded)
        );
        for surface in [0, Fluid::FULL_SURFACE + 1] {
            assert_eq!(
                tables.vet_occupants(ok(Voxel::AIR, Some(Fluid { surface, ..water }), None)),
                Err(PlaceFault::BadSurface),
                "{surface}"
            );
        }
        let shallow_source = Fluid {
            surface: 3,
            ..water
        };
        let reaching_source = Fluid { reach: 2, ..water };
        for source in [shallow_source, reaching_source] {
            assert_eq!(
                tables.vet_occupants(ok(Voxel::AIR, Some(source), None)),
                Err(PlaceFault::BadSurface),
                "{source:?}"
            );
        }
        assert_eq!(
            tables.vet_occupants(ok(Voxel::AIR, Some(water), Some(Burning { fuel: 1 }))),
            Err(PlaceFault::NoFuel)
        );
    }

    /// Flow in a Shape is stored per void: wet only where the solid has a
    /// void, dry levels carrying nothing, and never dry everywhere.
    #[test]
    fn vet_occupants_holds_flow_to_the_solids_voids() {
        let tables = Tables::builtin();
        let stair = Voxel::new(STONE, crate::SHAPE_STAIR, 0);
        let level = Level {
            surface: 12,
            reach: 3,
            descent: 0,
        };
        let flow = |levels| Fluid::flowing(WATER, levels).unwrap();
        let ok = |solid, fluid: Fluid| {
            tables.vet_occupants(Occupants {
                solid,
                fluid: Some(fluid),
                burning: None,
            })
        };
        let dry = Level::DRY;
        assert_eq!(ok(stair, flow([level, dry, dry, dry])), Ok(()));
        assert_eq!(ok(stair, Fluid::source(WATER)), Ok(()));
        assert_eq!(
            ok(stair, flow([level, level, dry, dry])),
            Err(PlaceFault::BadSurface),
            "a stair has one void"
        );
        assert_eq!(
            ok(Voxel::AIR, flow([dry, level, dry, dry])),
            Err(PlaceFault::BadSurface),
            "empty space's one void is the first"
        );
        let stale_dry = Level { reach: 1, ..dry };
        assert_eq!(
            ok(stair, flow([level, stale_dry, dry, dry])),
            Err(PlaceFault::BadSurface)
        );
        let all_dry = Fluid {
            source: false,
            surface: 0,
            ..Fluid::source(WATER)
        };
        assert_eq!(ok(stair, all_dry), Err(PlaceFault::BadSurface));
    }

    /// One light value per Voxel: the solid's seals, the larger dimming and
    /// the brighter emission.
    #[test]
    fn a_wet_solid_keeps_its_seals_and_takes_the_larger_dimming() {
        let tables = Tables::builtin();
        let stair = Voxel::new(STONE, crate::SHAPE_STAIR, 0);
        let (mask, dim) = tables.light_params(stair);
        assert_eq!(dim, 0);
        assert_eq!(
            tables.wet_light_params(stair, Some(WATER)),
            (mask, tables.materials.dimming(Voxel::cube(WATER)))
        );
        assert_eq!(
            tables.wet_light_params(Voxel::AIR, Some(WATER)),
            tables.light_params(Voxel::cube(WATER))
        );
        assert_eq!(tables.wet_light_params(stair, None), (mask, 0));
        let lantern = Voxel::new(crate::material::LANTERN, crate::SHAPE_STAIR, 0);
        assert_eq!(
            tables.wet_emission(lantern, Some(WATER)),
            tables.materials.emission(lantern)
        );
    }

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
