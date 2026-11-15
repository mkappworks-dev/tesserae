//! Every Mod's `templates`, from a data Declaration or a component alike,
//! resolved against the World's Materials and Shapes once all have declared.

use std::collections::HashMap;

use tesserae_core::shape::Orientation;
use tesserae_core::template::{Cell, DEFAULT_LEVEL_TOLERANCE_M, MAX_TEMPLATES, Template};
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE, ShapeId, Tables, Voxel};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, NAME_RULE, valid_local};
use crate::error::ModError;
use crate::registry::{Kind, Registry};

/// Every Mod's Template rows in load order, taken from its Behaviour before
/// [`crate::resolve`] consumes it.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DeclaredTemplates {
    rows: Vec<(String, Vec<content::TemplateDef>)>,
}

impl DeclaredTemplates {
    pub fn take(behaviours: &mut [Behaviour]) -> Self {
        Self {
            rows: behaviours
                .iter_mut()
                .map(|b| (b.mod_id.clone(), std::mem::take(&mut b.templates)))
                .collect(),
        }
    }

    /// The World's Templates in load order. `tables` must hold every Material
    /// and Shape already.
    pub fn resolve(&self, registry: &Registry, tables: &Tables) -> Result<Vec<Template>, ModError> {
        let mut templates: Vec<Template> = Vec::new();
        for (mod_id, rows) in &self.rows {
            for (row, def) in rows.iter().enumerate() {
                let fail = |reason: String| ModError::Template {
                    id: mod_id.clone(),
                    row,
                    name: def.name.clone(),
                    reason,
                };
                if templates.len() == MAX_TEMPLATES {
                    return Err(fail(format!(
                        "is one more than the {MAX_TEMPLATES} Templates a World holds"
                    )));
                }
                if !valid_local(&def.name) {
                    return Err(fail(NAME_RULE.into()));
                }
                let name = format!("{mod_id}:{}", def.name);
                if templates.iter().any(|t| t.name == name) {
                    return Err(fail("must be unique among the Mod's templates".into()));
                }
                templates.push(template(name, def, registry, tables).map_err(fail)?);
            }
        }
        Ok(templates)
    }
}

fn template(
    name: String,
    def: &content::TemplateDef,
    registry: &Registry,
    tables: &Tables,
) -> Result<Template, String> {
    let mut palette = HashMap::new();
    for entry in &def.palette {
        let at = format!("palette `{}`", entry.symbol);
        let cell = match &entry.cell {
            content::TemplateCell::Keep => Cell::Keep,
            content::TemplateCell::Air => Cell::Air,
            content::TemplateCell::Voxel(v) => {
                Cell::Voxel(voxel(v, registry, tables).map_err(|reason| format!("{at}: {reason}"))?)
            }
        };
        if palette.insert(entry.symbol, cell).is_some() {
            return Err(format!("{at} is declared twice"));
        }
    }
    let layers = def
        .layers
        .iter()
        .enumerate()
        .map(|(y, layer)| {
            layer
                .iter()
                .enumerate()
                .map(|(z, row)| {
                    row.chars()
                        .enumerate()
                        .map(|(x, symbol)| {
                            palette.get(&symbol).copied().ok_or_else(|| {
                                format!(
                                    "layer {y}, row {z}, column {x}: `{symbol}` is not in the \
                                     palette"
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ground_layer = def
        .ground_layer
        .ok_or("names no ground-layer, the layer that sits at ground height")?;
    let foundation = def
        .foundation
        .as_deref()
        .map(|m| solid(m, registry, tables).map_err(|reason| format!("foundation: {reason}")))
        .transpose()?;
    Template::new(
        name,
        def.cell_m,
        layers,
        ground_layer as usize,
        foundation,
        def.level_tolerance_m.unwrap_or(DEFAULT_LEVEL_TOLERANCE_M),
    )
    .map_err(|e| e.to_string())
}

fn voxel(
    v: &content::TemplateVoxel,
    registry: &Registry,
    tables: &Tables,
) -> Result<Voxel, String> {
    let material = solid(&v.material, registry, tables)?;
    let shape = match &v.shape {
        Some(name) => registry
            .id_of(Kind::Shape, name)
            .ok_or_else(|| format!("names unknown Shape `{name}`"))?
            as ShapeId,
        None => SHAPE_FULL_CUBE,
    };
    let orientation =
        Orientation::from_bits(v.orientation.unwrap_or(0)).ok_or("orientation must be below 24")?;
    let voxel = Voxel::shaped(material, shape, orientation);
    tables.vet_placed(voxel).map_err(|f| f.to_string())?;
    Ok(voxel)
}

/// A Material a Template may write: one that exists and is no fluid, since a
/// fluid is an occupant and air is the `air` cell.
fn solid(name: &str, registry: &Registry, tables: &Tables) -> Result<MaterialId, String> {
    let id = registry
        .id_of(Kind::Material, name)
        .ok_or_else(|| format!("names unknown Material `{name}`"))? as MaterialId;
    tables
        .vet_placed(Voxel::cube(id))
        .map_err(|f| format!("Material `{name}`: {f}"))?;
    if tables.fluids.contains(id) {
        return Err(format!(
            "Material `{name}` is a fluid, which a Template does not carry"
        ));
    }
    Ok(id)
}
