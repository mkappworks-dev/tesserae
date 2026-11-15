//! Marker kinds: the names, icons and colours a Game gives the marks on the Map and the Compass.

use serde::{Deserialize, Serialize};

/// The shapes the Engine draws a Marker kind as; closed so no font has to carry an icon.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Home,
    Camp,
    Mine,
    Farm,
    Landmark,
    Death,
    Flag,
    Pin,
    Star,
}

impl Icon {
    pub const ALL: [Icon; 9] = [
        Icon::Home,
        Icon::Camp,
        Icon::Mine,
        Icon::Farm,
        Icon::Landmark,
        Icon::Death,
        Icon::Flag,
        Icon::Pin,
        Icon::Star,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Icon::Home => "home",
            Icon::Camp => "camp",
            Icon::Mine => "mine",
            Icon::Farm => "farm",
            Icon::Landmark => "landmark",
            Icon::Death => "death",
            Icon::Flag => "flag",
            Icon::Pin => "pin",
            Icon::Star => "star",
        }
    }

    pub fn from_name(name: &str) -> Option<Icon> {
        Icon::ALL.into_iter().find(|i| i.name() == name)
    }
}

/// A declared Marker kind, named `<mod id>:<name>`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MarkerKind {
    pub name: String,
    pub icon: Icon,
    /// sRGB with straight alpha.
    pub color: [u8; 4],
}

/// Every Marker kind of a World in load order, and which one marks a death.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MarkerKinds {
    pub kinds: Vec<MarkerKind>,
    /// An index into `kinds`.
    pub death: Option<usize>,
}

impl MarkerKinds {
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.kinds.iter().position(|k| k.name == name)
    }
}

/// Waypoints a Player may hold, the death marker included.
pub const MAX_WAYPOINTS: usize = 64;

/// A name's length in characters.
pub const MAX_NAME_CHARS: usize = 32;

/// A named point on a Player's Map.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Waypoint {
    pub id: u32,
    pub name: String,
    /// A Marker kind's name; the kind may since have left the World's Game, so a reader tolerates a miss.
    pub kind: String,
    pub x_m: f32,
    pub z_m: f32,
    /// The Server's marker where the Player died; the Player cannot remove it.
    #[serde(default)]
    pub death: bool,
}

/// A Player's Waypoints in the order they were made.
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Waypoints {
    /// Never reused, so an id names one Waypoint for the life of the save.
    next_id: u32,
    list: Vec<Waypoint>,
}

impl Waypoints {
    pub fn list(&self) -> &[Waypoint] {
        &self.list
    }

    pub fn get(&self, id: u32) -> Option<&Waypoint> {
        self.list.iter().find(|w| w.id == id)
    }

    /// The name a blank one would get if a Waypoint were added now.
    pub fn next_default_name(&self) -> String {
        format!("Waypoint {}", self.next_id + 1)
    }

    /// Adds with the next id and returns it. A blank `name` becomes `Waypoint <id>`.
    pub fn push(&mut self, name: &str, kind: &str, x_m: f32, z_m: f32, death: bool) -> u32 {
        self.next_id += 1;
        let id = self.next_id;
        let name = match name.trim() {
            "" => format!("Waypoint {id}"),
            named => named.to_owned(),
        };
        self.list.push(Waypoint {
            id,
            name,
            kind: kind.to_owned(),
            x_m,
            z_m,
            death,
        });
        id
    }

    pub fn remove(&mut self, id: u32) -> Option<Waypoint> {
        let at = self.list.iter().position(|w| w.id == id)?;
        Some(self.list.remove(at))
    }

    /// Drops the death marker, so a new death replaces the old.
    pub fn clear_death(&mut self) {
        self.list.retain(|w| !w.death);
    }

    /// Drops the oldest ordinary Waypoint; the death marker must always fit, so it never meets the cap.
    pub fn drop_oldest_ordinary(&mut self) {
        if let Some(at) = self.list.iter().position(|w| !w.death) {
            self.list.remove(at);
        }
    }

    pub fn is_full(&self) -> bool {
        self.list.len() >= MAX_WAYPOINTS
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Whether no Waypoint was ever made, so there is nothing to save.
    pub fn is_pristine(&self) -> bool {
        self.list.is_empty() && self.next_id == 0
    }

    pub fn validate(&self) -> Result<(), crate::save::Error> {
        let sound = self.list.len() <= MAX_WAYPOINTS
            && self.list.iter().all(|w| {
                w.id <= self.next_id
                    && valid_name(&w.name)
                    && !w.name.trim().is_empty()
                    && w.x_m.is_finite()
                    && w.z_m.is_finite()
            })
            && self
                .list
                .iter()
                .enumerate()
                .all(|(i, w)| self.list[..i].iter().all(|o| o.id != w.id));
        if sound {
            Ok(())
        } else {
            Err(crate::save::Error::Corrupt("invalid Waypoints".into()))
        }
    }
}

/// Whether `name` is short enough and free of control characters.
pub fn valid_name(name: &str) -> bool {
    name.chars().count() <= MAX_NAME_CHARS && !name.chars().any(char::is_control)
}

/// A Player's Waypoints, saved with them.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WaypointsV1(pub Waypoints);

impl crate::save::PlayerExtension for WaypointsV1 {
    const KEY: &'static str = "waypoints_v1";

    fn validate(&self) -> Result<(), crate::save::Error> {
        self.0.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_round_trips_through_its_name() {
        for icon in Icon::ALL {
            assert_eq!(Icon::from_name(icon.name()), Some(icon));
        }
        assert_eq!(Icon::from_name("glyph"), None);
    }

    #[test]
    fn a_blank_name_becomes_the_default_and_ids_are_never_reused() {
        let mut w = Waypoints::default();
        let a = w.push("  ", "t:home", 1.0, 2.0, false);
        w.remove(a);
        let b = w.push("Base", "t:home", 1.0, 2.0, false);
        assert_eq!((a, b), (1, 2));
        w.push("", "t:home", 0.0, 0.0, false);
        assert_eq!(w.list()[1].name, "Waypoint 3");
        assert_eq!(w.next_default_name(), "Waypoint 4", "ids are not reused");
    }

    #[test]
    fn a_name_is_short_and_free_of_control_characters() {
        assert!(valid_name("North camp"));
        assert!(!valid_name("a\nb"));
        assert!(!valid_name(&"x".repeat(MAX_NAME_CHARS + 1)));
    }

    #[test]
    fn duplicate_ids_do_not_validate() {
        let mut w = Waypoints::default();
        w.push("A", "t:home", 0.0, 0.0, false);
        w.push("B", "t:home", 0.0, 0.0, false);
        let mut json = serde_json::to_value(&w).unwrap();
        json["list"][1]["id"] = 1.into();
        let bad: Waypoints = serde_json::from_value(json).unwrap();
        assert!(bad.validate().is_err());
    }
}
