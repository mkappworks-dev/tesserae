//! Every Mod's `settlements`, from a data Declaration or a component alike, resolved against the
//! World's Templates once all have declared.

use tesserae_core::settlement::{
    Building, DEFAULT_SETTLEMENT_MARGIN_M, MAX_SETTLEMENTS, Outskirts, PlotRule, Settlement,
};
use tesserae_core::shape::Orientation;
use tesserae_core::{MaterialId, SHAPE_FULL_CUBE, ShapeId, Tables, Voxel};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, ItemDef, NAME_RULE, valid_local};
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

    /// Refuses the first loot entry whose Item no loaded Mod declares, or whose stack is too big.
    pub fn check_loot(&self, items: &[ItemDef]) -> Result<(), ModError> {
        for (mod_id, rows) in &self.rows {
            for (row, def) in rows.iter().enumerate() {
                let fail = |reason: String| ModError::Settlement {
                    id: mod_id.clone(),
                    row,
                    name: def.name.clone(),
                    reason,
                };
                if let Some(loot) = &def.loot {
                    crate::loot::check(loot, items).map_err(|e| fail(format!("loot: {e}")))?;
                }
                let outskirts = def.outskirts.iter().flat_map(|o| &o.templates);
                for plot in def.templates.iter().chain(outskirts) {
                    if let Some(loot) = &plot.loot {
                        crate::loot::check(loot, items)
                            .map_err(|e| fail(format!("`{}` loot: {e}", plot.template)))?;
                    }
                }
            }
        }
        Ok(())
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
    let rules_of = |plots: &[content::SettlementPlot]| {
        plots
            .iter()
            .map(|p| {
                Ok(PlotRule {
                    building: building(&p.template)?,
                    weight: p.weight,
                    min: p.min,
                    max: p.max,
                    loot: p
                        .loot
                        .as_ref()
                        .map(crate::loot::table)
                        .transpose()
                        .map_err(|e| format!("`{}` loot: {e}", p.template))?,
                })
            })
            .collect::<Result<Vec<_>, String>>()
    };
    let rules = rules_of(&def.templates)?;
    let outskirts = match &def.outskirts {
        Some(o) => Outskirts {
            rules: rules_of(&o.templates).map_err(|e| format!("outskirts: {e}"))?,
            count: (o.count_min, o.count_max),
            chance: o.chance.unwrap_or(1.0),
        },
        None => Outskirts::default(),
    };
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
        radius_min_m: def.radius_min_m.unwrap_or(def.radius_m),
        margin_m: def.margin_m.unwrap_or(DEFAULT_SETTLEMENT_MARGIN_M),
        centre: def.centre.as_deref().map(building).transpose()?,
        plots: def.plots,
        plots_min: def.plots_min.unwrap_or(def.plots),
        rules,
        minimum: def.minimum,
        path,
        loot: def
            .loot
            .as_ref()
            .map(crate::loot::table)
            .transpose()
            .map_err(|e| format!("loot: {e}"))?,
        outskirts,
    };
    settlement.validate(&tables.templates)?;
    Ok(settlement)
}
