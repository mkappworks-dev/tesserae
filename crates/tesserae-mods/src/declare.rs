//! A Declaration into `tesserae-core`'s tables and the Registry.

use std::collections::HashSet;

use tesserae_core::{ParamError, Tables, WorldgenParams};

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::error::ModError;
use crate::registry::{Kind, Registry};

/// The palette holds 64 colours and 8 are built in (`PALETTE_LEN`).
pub const MAX_MATERIALS: usize = 56;
/// A `ShapeId` is a `u8` and 4 are built in.
pub const MAX_SHAPES: usize = 252;
pub const MAX_PER_MOD: usize = 64;
pub const MAX_NAME_BYTES: usize = 32;

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

/// What the Game chose that is not a table row.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Applied {
    pub world: Option<WorldgenParams>,
    pub view: Option<ViewParams>,
}

/// Validates `declaration` and registers it after whatever `tables` and
/// `registry` already hold, in declaration order. A load that fails is
/// abandoned, so an `Err` can leave a partly registered Mod behind.
pub fn apply(
    declaration: Declaration,
    mod_id: &str,
    tables: &mut Tables,
    registry: &mut Registry,
    is_root: bool,
) -> Result<Applied, ModError> {
    debug_assert_eq!(tables.materials.len(), registry.materials.len());
    debug_assert_eq!(tables.shapes.len(), registry.shapes.len());
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

    let world = declaration.world.map(WorldgenParams::from);
    if let Some(params) = &world {
        params.validate().map_err(|source| ModError::Params {
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

    for (row, material) in declaration.materials.into_iter().enumerate() {
        let name = qualified(mod_id, Kind::Material, row, &material.name)?;
        tables
            .materials
            .register(material.into())
            .map_err(|source| ModError::Material {
                id: id(),
                row,
                name: name.clone(),
                source,
            })?;
        registry.materials.push(name);
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
        registry.shapes.push(name);
    }
    Ok(Applied { world, view })
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
    let (total_max, held) = match kind {
        Kind::Material => (MAX_MATERIALS, registry.materials.len()),
        Kind::Shape => (MAX_SHAPES, registry.shapes.len()),
    };
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
