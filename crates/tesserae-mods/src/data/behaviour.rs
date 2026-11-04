use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Heat {
    strength: u8,
    reach_m: f32,
}

impl From<Heat> for content::HeatDef {
    fn from(h: Heat) -> Self {
        Self {
            strength: h.strength,
            reach_m: h.reach_m,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Fluid {
    material: String,
    reach_m: f32,
    sideways_m_per_s: f32,
    downward_m_per_s: f32,
    recession_m_per_s: f32,
    reset_drop_m: f32,
    waterlogs: bool,
    extinguishes: bool,
    heat: Option<Heat>,
}

impl From<Fluid> for content::FluidDef {
    fn from(f: Fluid) -> Self {
        Self {
            material: f.material,
            reach_m: f.reach_m,
            sideways_m_per_s: f.sideways_m_per_s,
            downward_m_per_s: f.downward_m_per_s,
            recession_m_per_s: f.recession_m_per_s,
            reset_drop_m: f.reset_drop_m,
            waterlogs: f.waterlogs,
            extinguishes: f.extinguishes,
            heat: f.heat.map(Into::into),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum State {
    Any,
    Source,
    Flowing,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Side {
    First,
    Second,
}

impl From<Side> for content::ReactionSide {
    fn from(side: Side) -> Self {
        match side {
            Side::First => Self::First,
            Side::Second => Self::Second,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reactant {
    fluid: String,
    state: State,
}

impl From<Reactant> for content::Reactant {
    fn from(r: Reactant) -> Self {
        Self {
            fluid: r.fluid,
            state: match r.state {
                State::Any => content::FluidState::Any,
                State::Source => content::FluidState::Source,
                State::Flowing => content::FluidState::Flowing,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reaction {
    first: Reactant,
    second: Reactant,
    consumed: Side,
    product: Option<String>,
    receiving: Option<Side>,
}

impl From<Reaction> for content::ReactionDef {
    fn from(r: Reaction) -> Self {
        Self {
            first: r.first.into(),
            second: r.second.into(),
            consumed: r.consumed.into(),
            product: r.product,
            receiving: r.receiving.map(Into::into),
        }
    }
}

/// `end = "permanent"` or `end = { after = { seconds = 8, residue = "m:ash" } }`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
enum End {
    Permanent,
    After(After),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct After {
    seconds: f32,
    residue: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Combustion {
    fuel: String,
    ignition_threshold: u8,
    ignition_delay_s: f32,
    emission: u8,
    heat: Option<Heat>,
    end: End,
}

impl From<Combustion> for content::CombustionDef {
    fn from(c: Combustion) -> Self {
        Self {
            fuel: c.fuel,
            ignition_threshold: c.ignition_threshold,
            ignition_delay_s: c.ignition_delay_s,
            emission: c.emission,
            heat: c.heat.map(Into::into),
            end: match c.end {
                End::Permanent => content::BurnEnd::Permanent,
                End::After(a) => content::BurnEnd::After(content::BurnAfter {
                    seconds: a.seconds,
                    residue: a.residue,
                }),
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    Measure,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tool {
    name: String,
    kind: Kind,
}

impl From<Tool> for content::ToolDef {
    fn from(t: Tool) -> Self {
        Self {
            name: t.name,
            kind: match t.kind {
                Kind::Measure => content::ToolKind::Measure,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Item {
    name: String,
    size_m: f32,
    color: [u8; 4],
    material: Option<String>,
}

impl From<Item> for content::ItemDef {
    fn from(i: Item) -> Self {
        let [r, g, b, a] = i.color;
        Self {
            name: i.name,
            size_m: i.size_m,
            color: crate::bindings::tesserae::mods::types::Srgba { r, g, b, a },
            material: i.material,
        }
    }
}
