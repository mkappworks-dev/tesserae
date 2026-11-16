//! Loot tables, on a Settlement, a Settlement's Template row or a Biome. Each Item a table names is
//! checked once every Item exists, implicit ones included; it need not be Obtainable another way.

use tesserae_core::loot::{LootEntry, LootTable};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, ItemDef};
use crate::error::ModError;

pub(crate) fn table(def: &content::LootTable) -> Result<LootTable, String> {
    let table = LootTable {
        rolls_min: def.rolls_min,
        rolls_max: def.rolls_max,
        entries: def
            .entries
            .iter()
            .map(|e| LootEntry {
                item: e.item.clone(),
                min: e.min,
                max: e.max,
                weight: e.weight,
            })
            .collect(),
    };
    table.validate()?;
    Ok(table)
}

/// Refuses an entry whose Item no loaded Mod declares, or whose `max` is over the Item's stack.
pub(crate) fn check(def: &content::LootTable, items: &[ItemDef]) -> Result<(), String> {
    for entry in &def.entries {
        let item = items
            .iter()
            .find(|i| i.name == entry.item)
            .ok_or_else(|| format!("`{}` is no Item a loaded Mod declares", entry.item))?;
        if entry.max > item.max_stack {
            return Err(format!(
                "`{}`: max {} is over the Item's stack of {}",
                entry.item, entry.max, item.max_stack
            ));
        }
    }
    Ok(())
}

/// Each Biome's loot, by the id [`crate::resolve_biomes`] gives it from the same rows.
pub fn biome_loot(
    behaviours: &[Behaviour],
    items: &[ItemDef],
) -> Result<Vec<Option<LootTable>>, ModError> {
    let mut loot = Vec::new();
    for b in behaviours {
        for def in &b.biomes {
            let fail = |rule: String| ModError::BadLoot {
                id: b.mod_id.clone(),
                biome: def.name.clone(),
                rule,
            };
            loot.push(
                def.loot
                    .as_ref()
                    .map(|l| {
                        check(l, items).map_err(fail)?;
                        table(l).map_err(fail)
                    })
                    .transpose()?,
            );
        }
    }
    Ok(loot)
}
