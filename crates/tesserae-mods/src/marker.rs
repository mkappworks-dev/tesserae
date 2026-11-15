//! Every Mod's `marker-kinds`, resolved into the World's table once all have declared.

use tesserae_core::marker::{Icon, MarkerKind, MarkerKinds};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, NAME_RULE, valid_local};
use crate::error::ModError;

pub const MAX_MARKER_KINDS_PER_MOD: usize = 16;

pub fn resolve_markers(behaviours: &[Behaviour]) -> Result<MarkerKinds, ModError> {
    let mut table = MarkerKinds::default();
    for b in behaviours {
        for (row, def) in b.marker_kinds.iter().enumerate() {
            let fail = |reason: &str| ModError::Marker {
                id: b.mod_id.clone(),
                row,
                name: def.name.clone(),
                reason: reason.into(),
            };
            if !valid_local(&def.name) {
                return Err(fail(NAME_RULE));
            }
            let name = format!("{}:{}", b.mod_id, def.name);
            if table.index_of(&name).is_some() {
                return Err(fail("must be unique among the Mod's marker kinds"));
            }
            table.kinds.push(MarkerKind {
                name,
                icon: icon(def.icon),
                color: [def.color.r, def.color.g, def.color.b, def.color.a],
            });
        }
    }
    for b in behaviours {
        if let Some(name) = &b.death_marker_kind {
            let death = table.index_of(name).ok_or_else(|| ModError::DeathMarker {
                id: b.mod_id.clone(),
                name: name.clone(),
            })?;
            table.death = Some(death);
        }
    }
    Ok(table)
}

fn icon(icon: content::MarkerIcon) -> Icon {
    match icon {
        content::MarkerIcon::Home => Icon::Home,
        content::MarkerIcon::Camp => Icon::Camp,
        content::MarkerIcon::Mine => Icon::Mine,
        content::MarkerIcon::Farm => Icon::Farm,
        content::MarkerIcon::Landmark => Icon::Landmark,
        content::MarkerIcon::Death => Icon::Death,
        content::MarkerIcon::Flag => Icon::Flag,
        content::MarkerIcon::Pin => Icon::Pin,
        content::MarkerIcon::Star => Icon::Star,
    }
}
