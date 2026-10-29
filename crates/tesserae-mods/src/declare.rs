//! A Declaration into `tesserae-core`'s tables and the Registry.

use std::collections::HashSet;
use std::time::Duration;

pub const DEFAULT_REMOVE_TIME: Duration = Duration::from_millis(350);

use tesserae_core::{
    AIR, Burn, CombustionDef, FluidDef, GroundParams, Heat, MaterialId, ParamError, Phase,
    Reactant, ReactionDef, Side, Tables, WaterParams, WorldgenParams,
};

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::convert::world_params;
use crate::error::ModError;
use crate::registry::{Kind, Registry};

/// The palette has 64 slots; ids 0 through 7 are reserved (`PALETTE_LEN`).
pub const MAX_MATERIALS: usize = 56;
/// A `ShapeId` is a `u8` and 4 are built in.
pub const MAX_SHAPES: usize = 252;
pub const MAX_PER_MOD: usize = 64;
/// Appearance, fluid, reaction and fire rows one Mod may declare, each.
pub const MAX_BEHAVIOURS_PER_MOD: usize = 64;
pub const MAX_NAME_BYTES: usize = 32;
pub const MATERIAL_TEXTURE_EDGE: usize = 16;
pub const MATERIAL_TEXTURE_BYTES: usize = MATERIAL_TEXTURE_EDGE * MATERIAL_TEXTURE_EDGE * 3 * 4;

/// The Game's view distance and level-of-detail reach, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ViewParams {
    pub view_distance_m: f32,
    pub lod_distance_m: f32,
}

impl ViewParams {
    pub const VIEW_RANGE_M: (f32, f32) = (32.0, 512.0);
    pub const MAX_LOD_M: f32 = 8192.0;

    /// A `lod_distance_m` equal to the view distance means no coarse levels.
    pub fn validate(&self) -> Result<(), ParamError> {
        let (min, max) = Self::VIEW_RANGE_M;
        if !(min..=max).contains(&self.view_distance_m) {
            return Err(ParamError {
                field: "view_distance_m",
                rule: "must be finite and from 32 to 512 m",
            });
        }
        if !(self.view_distance_m..=Self::MAX_LOD_M).contains(&self.lod_distance_m) {
            return Err(ParamError {
                field: "lod_distance_m",
                rule: "must be finite and from view_distance_m to 8192 m",
            });
        }
        Ok(())
    }
}

impl From<content::ViewParams> for ViewParams {
    fn from(p: content::ViewParams) -> Self {
        ViewParams {
            view_distance_m: p.view_distance_m,
            lod_distance_m: p.lod_distance_m,
        }
    }
}

/// What the Game chose that is not a table row, and the rows that wait for
/// every Mod's names before [`resolve`] registers them.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Applied {
    pub remove_times: Vec<(MaterialId, Duration)>,
    /// Validated appearance data, transferred without copying texture pixels.
    pub appearance: Vec<content::Appearance>,
    /// With no lake: [`resolve`] returns the one it names.
    pub world: Option<WorldgenParams>,
    pub view: Option<ViewParams>,
    pub movement: Option<content::MovementPolicy>,
    pub character: Option<crate::character::Character>,
    pub behaviour: Behaviour,
}

/// One Mod's fluid, reaction and fire rows and the Game's lake, by full
/// Material name.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Behaviour {
    mod_id: String,
    fluids: Vec<content::FluidDef>,
    reactions: Vec<content::ReactionDef>,
    combustion: Vec<content::CombustionDef>,
    lake: Option<content::WaterParams>,
    ground: Option<content::GroundParams>,
}

/// Validates `declaration` and registers it after whatever `tables` and
/// `registry` already hold, in declaration order. A load that fails is
/// abandoned, so an `Err` can leave a partly registered Mod behind.
pub fn apply(
    mut declaration: Declaration,
    mod_id: &str,
    tables: &mut Tables,
    registry: &mut Registry,
    is_root: bool,
) -> Result<Applied, ModError> {
    debug_assert_eq!(tables.materials.len(), registry.len(Kind::Material));
    debug_assert_eq!(tables.shapes.len(), registry.len(Kind::Shape));
    let id = || mod_id.to_owned();

    if mod_id == CORE {
        return Err(ModError::ReservedId { id: id() });
    }
    if !is_root {
        for (set, field) in [
            (declaration.world.is_some(), "world"),
            (declaration.view.is_some(), "view"),
        ] {
            if set {
                return Err(ModError::NotAGame { id: id(), field });
            }
        }
    }

    for (what, count) in [
        ("appearance", declaration.appearance.len()),
        ("fluid", declaration.fluids.len()),
        ("reaction", declaration.reactions.len()),
        ("combustion", declaration.combustion.len()),
    ] {
        if count > MAX_BEHAVIOURS_PER_MOD {
            return Err(ModError::TooManyRows {
                id: id(),
                what,
                count,
                max: MAX_BEHAVIOURS_PER_MOD,
            });
        }
    }
    for (row, appearance) in declaration.appearance.iter().enumerate() {
        if let Some(pixels) = &appearance.texture
            && pixels.len() != MATERIAL_TEXTURE_BYTES
        {
            return Err(ModError::MaterialTexture {
                id: id(),
                row,
                name: appearance.material.clone(),
                bytes: pixels.len(),
            });
        }
    }
    check_count(
        mod_id,
        Kind::Material,
        declaration.materials.iter().map(|m| m.name.as_str()),
        registry,
    )?;
    check_count(
        mod_id,
        Kind::Shape,
        declaration.shapes.iter().map(|s| s.name.as_str()),
        registry,
    )?;

    let character = declaration
        .world
        .as_mut()
        .and_then(|p| p.character.take())
        .map(crate::character::validate)
        .transpose()
        .map_err(|source| ModError::Params {
            id: id(),
            what: "character",
            source,
        })?;
    let movement = declaration.world.as_ref().and_then(|p| p.movement);
    if let Some(p) = movement
        && ((p.start_in_flight && !p.allow_flight) || (!p.start_in_flight && !p.allow_physics))
    {
        return Err(ModError::Params {
            id: id(),
            what: "movement",
            source: ParamError {
                field: "initial mode",
                rule: "must be allowed",
            },
        });
    }
    let ground = declaration.world.as_ref().and_then(|p| p.ground.clone());
    let (world, lake) = declaration.world.map(world_params).unzip();
    let lake = lake.flatten();
    if let Some(params) = world {
        // `validate` reads only the lake's level; its fluid waits for `resolve`.
        let water = lake.as_ref().map(|l| WaterParams {
            level_m: l.level_m,
            material: AIR,
        });
        WorldgenParams { water, ..params }
            .validate()
            .map_err(|source| ModError::Params {
                id: id(),
                what: "world",
                source,
            })?;
    }
    let view = declaration.view.map(ViewParams::from);
    if let Some(params) = &view {
        params.validate().map_err(|source| ModError::Params {
            id: id(),
            what: "view",
            source,
        })?;
    }

    let mut remove_times = Vec::new();
    for (row, material) in declaration.materials.into_iter().enumerate() {
        let name = qualified(mod_id, Kind::Material, row, &material.name)?;
        let seconds = material.remove_time_s.unwrap_or(0.35);
        if !(0.05..=60.0).contains(&seconds) {
            return Err(ModError::RemoveTime {
                id: id(),
                row,
                name,
                seconds,
            });
        }
        let duration = material
            .remove_time_s
            .map_or(DEFAULT_REMOVE_TIME, Duration::from_secs_f32);
        let material_id = tables
            .materials
            .register(material.into())
            .map_err(|source| ModError::Material {
                id: id(),
                row,
                name: name.clone(),
                source,
            })?;
        remove_times.push((material_id, duration));
        registry.push(Kind::Material, name);
    }
    for (row, shape) in declaration.shapes.into_iter().enumerate() {
        let name = qualified(mod_id, Kind::Shape, row, &shape.name)?;
        tables
            .shapes
            .register(shape.into())
            .map_err(|source| ModError::Shape {
                id: id(),
                row,
                name: name.clone(),
                source,
            })?;
        registry.push(Kind::Shape, name);
    }
    Ok(Applied {
        remove_times,
        appearance: declaration.appearance,
        world,
        view,
        movement,
        character,
        behaviour: Behaviour {
            mod_id: id(),
            fluids: declaration.fluids,
            reactions: declaration.reactions,
            combustion: declaration.combustion,
            lake,
            ground,
        },
    })
}

/// Registers every Mod's fluids, then reactions, then fires, in load order,
/// once every Declaration is in: a row may name any Mod's Material. Fluids
/// come first because a reaction is between fluids. Returns the Game's lake,
/// last because any declared fluid may fill it. The tables are complete and
/// immutable when this returns; the World opens after it.
pub fn resolve(
    behaviours: Vec<Behaviour>,
    tables: &mut Tables,
    registry: &Registry,
) -> Result<(Option<GroundParams>, Option<WaterParams>), ModError> {
    for b in &behaviours {
        for (row, fluid) in b.fluids.iter().enumerate() {
            let material = lookup(b, registry, "fluid", row, &fluid.material)?;
            let heat = fluid.heat.map(heat);
            let def = FluidDef {
                reach_m: fluid.reach_m,
                sideways_m_per_s: fluid.sideways_m_per_s,
                downward_m_per_s: fluid.downward_m_per_s,
                recession_m_per_s: fluid.recession_m_per_s,
                reset_drop_m: fluid.reset_drop_m,
                waterlogs: fluid.waterlogs,
                extinguishes: fluid.extinguishes,
                heat,
            };
            tables
                .register_fluid(material, def)
                .map_err(|source| ModError::Fluid {
                    id: b.mod_id.clone(),
                    row,
                    name: fluid.material.clone(),
                    source,
                })?;
        }
    }
    for b in &behaviours {
        for (row, reaction) in b.reactions.iter().enumerate() {
            let reactant = |r: &content::Reactant| -> Result<Reactant, ModError> {
                Ok(Reactant {
                    fluid: lookup(b, registry, "reaction", row, &r.fluid)?,
                    phase: match r.state {
                        content::FluidState::Any => Phase::Any,
                        content::FluidState::Source => Phase::Source,
                        content::FluidState::Flowing => Phase::Flowing,
                    },
                })
            };
            let def = ReactionDef {
                first: reactant(&reaction.first)?,
                second: reactant(&reaction.second)?,
                consumed: side(reaction.consumed),
                product: reaction
                    .product
                    .as_deref()
                    .map(|p| lookup(b, registry, "reaction", row, p))
                    .transpose()?,
                receiving: reaction.receiving.map(side),
            };
            tables
                .register_reaction(def)
                .map_err(|source| ModError::Reaction {
                    id: b.mod_id.clone(),
                    row,
                    source,
                })?;
        }
    }
    for b in &behaviours {
        for (row, fire) in b.combustion.iter().enumerate() {
            let fuel = lookup(b, registry, "combustion", row, &fire.fuel)?;
            let burn = match &fire.end {
                content::BurnEnd::After(after) => Burn::For {
                    seconds: after.seconds,
                    residue: match &after.residue {
                        Some(name) => lookup(b, registry, "combustion", row, name)?,
                        None => AIR,
                    },
                },
                content::BurnEnd::Permanent => Burn::Permanent,
            };
            let def = CombustionDef {
                threshold: fire.ignition_threshold,
                ignition_delay_s: fire.ignition_delay_s,
                emission: fire.emission,
                heat: fire.heat.map(heat),
                burn,
            };
            tables
                .register_combustion(fuel, def)
                .map_err(|source| ModError::Combustion {
                    id: b.mod_id.clone(),
                    row,
                    name: fire.fuel.clone(),
                    source,
                })?;
        }
    }
    let mut ground = None;
    for b in &behaviours {
        if let Some(names) = &b.ground {
            let material = |field, name: &str| {
                registry
                    .id_of(Kind::Material, name)
                    .map(|id| id as MaterialId)
                    .ok_or_else(|| ModError::Params {
                        id: b.mod_id.clone(),
                        what: "world",
                        source: ParamError {
                            field,
                            rule: "names no registered Material",
                        },
                    })
            };
            let resolved = GroundParams {
                rock: material("ground.rock", &names.rock)?,
                soil: material("ground.soil", &names.soil)?,
                surface: material("ground.surface", &names.surface)?,
            };
            WorldgenParams {
                ground: Some(resolved),
                ..WorldgenParams::default()
            }
            .validate_ground(tables)
            .map_err(|source| ModError::Params {
                id: b.mod_id.clone(),
                what: "world",
                source,
            })?;
            ground = Some(resolved);
        }
    }
    let mut lake = None;
    for b in &behaviours {
        if let Some(water) = &b.lake {
            lake = Some(resolve_lake(b, water, ground, tables, registry)?);
        }
    }
    Ok((ground, lake))
}

/// The lake's fluid by id, refused as [`WorldgenParams::validate_lake`]
/// refuses it.
fn resolve_lake(
    b: &Behaviour,
    water: &content::WaterParams,
    ground: Option<GroundParams>,
    tables: &Tables,
    registry: &Registry,
) -> Result<WaterParams, ModError> {
    let refused = |source| ModError::Lake {
        id: b.mod_id.clone(),
        name: water.material.clone(),
        source,
    };
    let material = registry
        .id_of(Kind::Material, &water.material)
        .ok_or(ParamError {
            field: "water.material",
            rule: "names no registered Material",
        })
        .map_err(refused)? as MaterialId;
    let lake = WaterParams {
        level_m: water.level_m,
        material,
    };
    WorldgenParams {
        water: Some(lake),
        ground,
        ..WorldgenParams::default()
    }
    .validate_lake(tables)
    .map_err(refused)?;
    Ok(lake)
}

fn heat(h: content::HeatDef) -> Heat {
    Heat {
        strength: h.strength,
        reach_m: h.reach_m,
    }
}

fn side(s: content::ReactionSide) -> Side {
    match s {
        content::ReactionSide::First => Side::First,
        content::ReactionSide::Second => Side::Second,
    }
}

/// A full Material name's id. Ids are positions in a Registry the palette
/// bounds, so one fits a `MaterialId`.
fn lookup(
    b: &Behaviour,
    registry: &Registry,
    what: &'static str,
    row: usize,
    name: &str,
) -> Result<MaterialId, ModError> {
    registry
        .id_of(Kind::Material, name)
        .map(|id| id as MaterialId)
        .ok_or_else(|| ModError::UnknownName {
            id: b.mod_id.clone(),
            what,
            row,
            name: name.to_owned(),
        })
}

const CORE: &str = "core";

/// Every name in one pass, before anything is registered, so a bad name in the
/// last row costs the Mod no table rows.
fn check_count<'a>(
    mod_id: &str,
    kind: Kind,
    names: impl ExactSizeIterator<Item = &'a str>,
    registry: &Registry,
) -> Result<(), ModError> {
    let count = names.len();
    let total_max = match kind {
        Kind::Material => MAX_MATERIALS,
        Kind::Shape => MAX_SHAPES,
    };
    let held = registry.len(kind);
    if count > MAX_PER_MOD {
        return Err(ModError::TooMany {
            id: mod_id.to_owned(),
            kind,
            count,
            max: MAX_PER_MOD,
            scope: "per Mod",
        });
    }
    let total = held - Registry::builtin_len(kind) + count;
    if total > total_max {
        return Err(match kind {
            Kind::Material => ModError::PaletteFull {
                id: mod_id.to_owned(),
                count: total,
                max: total_max,
            },
            Kind::Shape => ModError::TooMany {
                id: mod_id.to_owned(),
                kind,
                count: total,
                max: total_max,
                scope: "in total",
            },
        });
    }
    let mut seen = HashSet::new();
    for (row, name) in names.enumerate() {
        let full = qualified(mod_id, kind, row, name)?;
        if registry.id_of(kind, &full).is_some() || !seen.insert(full.clone()) {
            return Err(ModError::DuplicateName {
                id: mod_id.to_owned(),
                kind,
                row,
                name: full,
            });
        }
    }
    Ok(())
}

/// `<mod id>:<local>`, for a local part of `[a-z][a-z0-9_]{0,31}`.
fn qualified(mod_id: &str, kind: Kind, row: usize, local: &str) -> Result<String, ModError> {
    let mut bytes = local.bytes();
    let well_formed = local.len() <= MAX_NAME_BYTES
        && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if well_formed {
        return Ok(format!("{mod_id}:{local}"));
    }
    Err(ModError::BadName {
        id: mod_id.to_owned(),
        kind,
        row,
        name: local.to_owned(),
        reason: if local.contains(':') {
            "a name is local to its Mod; the engine adds `<mod id>:`"
        } else {
            "must be a lowercase letter then up to 31 lowercase letters, digits or `_`"
        },
    })
}
