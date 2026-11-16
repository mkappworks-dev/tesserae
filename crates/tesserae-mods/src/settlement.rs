//! Every Mod's `settlements`, from a data Declaration or a component alike, resolved against the
//! World's Templates once all have declared.

use tesserae_core::settlement::{
    Building, DEFAULT_SETTLEMENT_MARGIN_M, MAX_SETTLEMENTS, PlotRule, Settlement,
};
use tesserae_core::shape::Orientation;
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE, ShapeId, Tables, Voxel};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, NAME_RULE, valid_local};
use crate::error::ModError;
use crate::registry::{Kind, Registry};
use crate::template::solid;

/// Every Mod's Settlement rows in load order, taken from its Behaviour before [`crate::resolve`]
/// consumes it.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DeclaredSettlements {
    rows: Vec<(String, Vec<content::SettlementDef>)>,
}

impl DeclaredSettlements {
    pub fn take(behaviours: &mut [Behaviour]) -> Self {
        Self {
            rows: behaviours
                .iter_mut()
                .map(|b| (b.mod_id.clone(), std::mem::take(&mut b.settlements)))
                .collect(),
        }
    }

    /// The World's Settlements in load order. `tables` must hold every Template already.
    pub fn resolve(
        &self,
        registry: &Registry,
        tables: &Tables,
    ) -> Result<Vec<Settlement>, ModError> {
        let mut settlements: Vec<Settlement> = Vec::new();
        for (mod_id, rows) in &self.rows {
            for (row, def) in rows.iter().enumerate() {
                let fail = |reason: String| ModError::Settlement {
                    id: mod_id.clone(),
                    row,
                    name: def.name.clone(),
                    reason,
                };
                if settlements.len() == MAX_SETTLEMENTS {
                    return Err(fail(format!(
                        "is one more than the {MAX_SETTLEMENTS} Settlements a World holds"
                    )));
                }
                if !valid_local(&def.name) {
                    return Err(fail(NAME_RULE.into()));
                }
                let name = format!("{mod_id}:{}", def.name);
                if settlements.iter().any(|s| s.name == name) {
                    return Err(fail("must be unique among the Mod's settlements".into()));
                }
                let settlement = settlement(name, def, registry, tables).map_err(fail)?;
                settlements.push(settlement);
            }
        }
        Ok(settlements)
    }

    /// Refuses the first path Material of a Settlement that is `used`, by its full name, that
    /// `refuse` gives a reason against, naming the Settlement.
    pub fn check(
        &self,
        registry: &Registry,
        used: impl Fn(&str) -> bool,
        refuse: impl Fn(MaterialId) -> Option<String>,
    ) -> Result<(), ModError> {
        for (mod_id, rows) in &self.rows {
            for (row, def) in rows.iter().enumerate() {
                if !used(&format!("{mod_id}:{}", def.name)) {
                    continue;
                }
                let Some(id) = registry.id_of(Kind::Material, &def.path) else {
                    continue;
                };
                if let Some(reason) = refuse(id as MaterialId) {
                    return Err(ModError::Settlement {
                        id: mod_id.clone(),
                        row,
                        name: def.name.clone(),
                        reason: format!("path: Material `{}` {reason}", def.path),
                    });
                }
            }
        }
        Ok(())
    }
}

fn settlement(
    name: String,
    def: &content::SettlementDef,
    registry: &Registry,
    tables: &Tables,
) -> Result<Settlement, String> {
    let building = |template: &str| -> Result<Building, String> {
        let at = tables
            .templates
            .iter()
            .position(|t| t.name == template)
            .ok_or_else(|| format!("`{template}` names no Template"))?;
        if tables.templates[at].spans() {
            return Err(format!(
                "`{template}` spans a gap, and a Plot is level ground"
            ));
        }
        let is_door = |m| tables.uses.door(m).is_some_and(|d| d.closed == m);
        Ok(Building {
            template: at as u16,
            door: tables.templates[at].door_cell(is_door),
        })
    };
    let rules = def
        .templates
        .iter()
        .map(|p| {
            Ok(PlotRule {
                building: building(&p.template)?,
                weight: p.weight,
                min: p.min,
                max: p.max,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let material = solid(&def.path, registry, tables).map_err(|e| format!("path: {e}"))?;
    let shape = match &def.path_shape {
        Some(shape) => registry
            .id_of(Kind::Shape, shape)
            .ok_or_else(|| format!("path-shape: names unknown Shape `{shape}`"))?
            as ShapeId,
        None => SHAPE_FULL_CUBE,
    };
    let path = Voxel::shaped(material, shape, Orientation::IDENTITY);
    tables.vet_placed(path).map_err(|f| format!("path: {f}"))?;
    let settlement = Settlement {
        name,
        radius_m: def.radius_m,
        margin_m: def.margin_m.unwrap_or(DEFAULT_SETTLEMENT_MARGIN_M),
        centre: def.centre.as_deref().map(building).transpose()?,
        plots: def.plots,
        rules,
        minimum: def.minimum,
        path,
    };
    settlement.validate(&tables.templates)?;
    Ok(settlement)
}
