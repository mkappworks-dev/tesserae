//! Everything a native consumer asks about what a Voxel is.

use crate::combustion::{Burn, CombustionDef, CombustionError, CombustionTable};
use crate::fluid::{FluidDef, FluidError, FluidTable};
use crate::light::MAX_LIGHT;
use crate::material::MaterialTable;
use crate::occupants::{Fluid, Level, Occupants};
use crate::reaction::{ReactionDef, ReactionError, ReactionTable};
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
    pub reactions: ReactionTable,
    pub combustion: CombustionTable,
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
    /// Burning of a solid no combustion row declares fuel.
    NotFuel,
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
            PlaceFault::NotFuel => "that Material does not burn",
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
            if slots == 0 || !self.holds(occupants.solid, fluid.material) {
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
        if occupants.burning.is_some() {
            if occupants.solid.is_air() {
                return Err(PlaceFault::NoFuel);
            }
            if !self.combustion.contains(occupants.solid.material()) {
                return Err(PlaceFault::NotFuel);
            }
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

    /// Registers what two fluids do on contact. Both must be fluids already
    /// and the product a solid Material; the rule is stored canonical.
    pub fn register_reaction(&mut self, def: ReactionDef) -> Result<(), ReactionError> {
        if !self.fluids.contains(def.first.fluid) || !self.fluids.contains(def.second.fluid) {
            return Err(ReactionError::NotAFluid);
        }
        if let Some(product) = def.product
            && !self.is_solid_material(product)
        {
            return Err(ReactionError::ProductNotSolid);
        }
        self.reactions.insert(def)
    }

    /// Registers how a solid Material burns. Its residue is air or a solid.
    pub fn register_combustion(
        &mut self,
        fuel: MaterialId,
        def: CombustionDef,
    ) -> Result<(), CombustionError> {
        let known = |m| self.materials.contains(m);
        if !known(fuel) {
            return Err(CombustionError::NotAMaterial);
        }
        if !self.is_solid_material(fuel) {
            return Err(CombustionError::NotSolid);
        }
        if let Burn::For { residue, .. } = def.burn
            && residue != crate::voxel::AIR
        {
            if !known(residue) {
                return Err(CombustionError::NotAMaterial);
            }
            if !self.is_solid_material(residue) {
                return Err(CombustionError::NotSolid);
            }
        }
        self.combustion.insert(fuel, def)
    }

    fn is_solid_material(&self, material: MaterialId) -> bool {
        self.materials.contains(material)
            && !self.fluids.contains(material)
            && self.materials.solid(Voxel::cube(material))
    }

    /// Whether fluid of `material` may stand beside `solid`: anywhere in
    /// empty space, and in a partial Shape's voids only if it waterlogs.
    pub fn holds(&self, solid: Voxel, material: MaterialId) -> bool {
        solid.is_air() || self.fluids.get(material).is_some_and(|f| f.waterlogs)
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

    /// The block light a solid gives off while it burns, 0 for one that does
    /// not burn. Combined with [`Tables::wet_emission`] by the larger, as two
    /// independent emitters in one Voxel are.
    #[inline]
    pub fn burning_emission(&self, voxel: Voxel) -> u8 {
        if voxel.is_air() {
            return 0;
        }
        self.combustion
            .get(voxel.material())
            .map_or(0, |def| def.emission)
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
    use crate::testgen::fixtures;
    use super::*;
    use crate::fluid::{FluidDef, FluidError};
    use crate::material::{MaterialDef, Occlusion, RenderClass};
    use crate::occupants::{Burning, Fluid, Occupants};
    use crate::testgen::{WATER_FLUID};

    #[test]
    fn a_fluid_is_registered_over_a_non_solid_material_only() {
        let mut tables = fixtures().tables.clone();
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
            tables.register_fluid(999, WATER_FLUID),
            Err(FluidError::NotAMaterial)
        );
        assert_eq!(
            tables.register_fluid(fixtures().ground.rock, WATER_FLUID),
            Err(FluidError::Solid)
        );
        assert_eq!(tables.register_fluid(decor, WATER_FLUID), Ok(()));
        assert!(tables.fluids.contains(decor));
    }

    /// What the shared Edit path refuses of a reading before it is stored.
    #[test]
    fn vet_occupants_names_the_first_thing_wrong() {
        let mut tables = fixtures().tables.clone();
        let water_id = fixtures().water;
        let water = Fluid::source(water_id);
        let ok = |solid, fluid, burning| Occupants {
            solid,
            fluid,
            burning,
        };
        let stair = Voxel::new(fixtures().ground.rock, crate::SHAPE_STAIR, 0);
        assert_eq!(
            tables.vet_occupants(ok(Voxel::AIR, Some(water), None)),
            Ok(())
        );
        assert_eq!(
            tables.vet_occupants(ok(stair, Some(water), Some(Burning { fuel: 1 }))),
            Err(PlaceFault::NotFuel),
            "stone is not declared fuel"
        );
        let mut burns = tables.clone();
        burns
            .register_combustion(fixtures().leaves, crate::combustion::tests::WOOD)
            .unwrap();
        let leafy = Voxel::new(fixtures().leaves, crate::SHAPE_STAIR, 0);
        assert_eq!(
            burns.vet_occupants(ok(leafy, Some(water), Some(Burning { fuel: 1 }))),
            Ok(())
        );
        assert_eq!(tables.vet_occupants(ok(Voxel::AIR, None, None)), Ok(()));
        assert_eq!(
            tables.vet_occupants(ok(Voxel::new(fixtures().ground.rock, 0, 24), None, None)),
            Err(PlaceFault::BadOrientation)
        );
        assert_eq!(
            tables.vet_occupants(ok(Voxel::cube(water_id), None, None)),
            Err(PlaceFault::NotSolid)
        );
        assert_eq!(
            tables.vet_occupants(ok(Voxel::AIR, Some(Fluid::source(fixtures().ground.rock)), None)),
            Err(PlaceFault::NotAFluid)
        );
        assert_eq!(
            tables.vet_occupants(ok(Voxel::cube(fixtures().ground.rock), Some(water), None)),
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
        let mut tables = fixtures().tables.clone();
        let water = fixtures().water;
        let stair = Voxel::new(fixtures().ground.rock, crate::SHAPE_STAIR, 0);
        let level = Level {
            surface: 12,
            reach: 3,
            descent: 0,
        };
        let flow = |levels| Fluid::flowing(water, levels).unwrap();
        let ok = |solid, fluid: Fluid| {
            tables.vet_occupants(Occupants {
                solid,
                fluid: Some(fluid),
                burning: None,
            })
        };
        let dry = Level::DRY;
        assert_eq!(ok(stair, flow([level, dry, dry, dry])), Ok(()));
        assert_eq!(ok(stair, Fluid::source(water)), Ok(()));
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
            ..Fluid::source(water)
        };
        assert_eq!(ok(stair, all_dry), Err(PlaceFault::BadSurface));
    }

    /// One light value per Voxel: the solid's seals, the larger dimming and
    /// the brighter emission.
    #[test]
    fn a_wet_solid_keeps_its_seals_and_takes_the_larger_dimming() {
        let mut tables = fixtures().tables.clone();
        let water = fixtures().water;
        let stair = Voxel::new(fixtures().ground.rock, crate::SHAPE_STAIR, 0);
        let (mask, dim) = tables.light_params(stair);
        assert_eq!(dim, 0);
        assert_eq!(
            tables.wet_light_params(stair, Some(water)),
            (mask, tables.materials.dimming(Voxel::cube(water)))
        );
        assert_eq!(
            tables.wet_light_params(Voxel::AIR, Some(water)),
            tables.light_params(Voxel::cube(water))
        );
        assert_eq!(tables.wet_light_params(stair, None), (mask, 0));
        let lantern = Voxel::new(fixtures().lantern, crate::SHAPE_STAIR, 0);
        assert_eq!(
            tables.wet_emission(lantern, Some(water)),
            tables.materials.emission(lantern)
        );
    }

    #[test]
    fn vet_placed_names_the_first_thing_wrong() {
        let tables = fixtures().tables.clone();
        let materials = tables.materials.len() as u16;
        let shapes = tables.shapes.len() as u8;
        assert_eq!(tables.vet_placed(Voxel::cube(fixtures().ground.rock)), Ok(()));
        assert_eq!(tables.vet_placed(Voxel::AIR), Err(PlaceFault::Air));
        assert_eq!(
            tables.vet_placed(Voxel::new(materials, 0, 0)),
            Err(PlaceFault::UnknownMaterial)
        );
        assert_eq!(
            tables.vet_placed(Voxel::new(fixtures().ground.rock, shapes, 0)),
            Err(PlaceFault::UnknownShape)
        );
        assert_eq!(
            tables.vet_placed(Voxel::new(fixtures().ground.rock, 0, 24)),
            Err(PlaceFault::BadOrientation)
        );
        assert_eq!(tables.vet_placed(Voxel::new(fixtures().ground.rock, 0, 23)), Ok(()));
    }

    /// Reactions and fires name Materials the tables hold, of the right kind.
    #[test]
    fn reactions_and_fires_refuse_materials_of_the_wrong_kind() {
        use crate::combustion::{Burn, CombustionDef, CombustionError};

        use crate::reaction::{Phase, Reactant, ReactionDef, ReactionError, Side};
        let mut tables = fixtures().tables.clone();
        let water = fixtures().water;
        let oil = tables
            .materials
            .register(MaterialDef {
                solid: false,
                emission: 0,
                dimming: 4,
                occlusion: Occlusion::SameMaterial,
                render: RenderClass::Blend,
            })
            .unwrap();
        let fluid = |fluid| Reactant {
            fluid,
            phase: Phase::Any,
        };
        let rule = |first, second, product| ReactionDef {
            first: fluid(first),
            second: fluid(second),
            consumed: Side::First,
            product,
            receiving: None,
        };
        assert_eq!(
            tables.register_reaction(rule(oil, water, None)),
            Err(ReactionError::NotAFluid)
        );
        tables.register_fluid(oil, WATER_FLUID).unwrap();
        for product in [water, oil, 999, 0] {
            assert_eq!(
                tables.register_reaction(rule(oil, water, Some(product))),
                Err(ReactionError::ProductNotSolid),
                "{product}"
            );
        }
        assert_eq!(
            tables.register_reaction(rule(oil, water, Some(fixtures().ground.rock))),
            Ok(())
        );
        assert_eq!(tables.reactions.rules().len(), 1);

        let burns = |residue| CombustionDef {
            threshold: 6,
            ignition_delay_s: 2.0,
            emission: 12,
            heat: None,
            burn: Burn::For {
                seconds: 8.0,
                residue,
            },
        };
        for (fuel, residue, error) in [
            (999, 0, CombustionError::NotAMaterial),
            (water, 0, CombustionError::NotSolid),
            (0, 0, CombustionError::NotSolid),
            (fixtures().leaves, 999, CombustionError::NotAMaterial),
            (fixtures().leaves, oil, CombustionError::NotSolid),
        ] {
            assert_eq!(
                tables.register_combustion(fuel, burns(residue)),
                Err(error),
                "{fuel} to {residue}"
            );
        }
        assert_eq!(tables.register_combustion(fixtures().leaves, burns(0)), Ok(()));
        assert_eq!(tables.register_combustion(fixtures().glass, burns(fixtures().ground.rock)), Ok(()));
        assert!(tables.combustion.contains(fixtures().leaves));
    }

    #[test]
    fn a_fluid_that_does_not_waterlog_is_held_by_empty_space_only() {
        let mut tables = fixtures().tables.clone();
        let water = fixtures().water;
        let tar = tables
            .materials
            .register(MaterialDef {
                solid: false,
                emission: 0,
                dimming: 15,
                occlusion: Occlusion::Opaque,
                render: RenderClass::Opaque,
            })
            .unwrap();
        tables
            .register_fluid(
                tar,
                FluidDef {
                    waterlogs: false,
                    ..WATER_FLUID
                },
            )
            .unwrap();
        let stair = Voxel::new(fixtures().ground.rock, crate::SHAPE_STAIR, 0);
        assert!(tables.holds(Voxel::AIR, tar));
        assert!(!tables.holds(stair, tar));
        assert!(tables.holds(stair, water));
    }
}
