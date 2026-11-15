//! Marker kinds: the names, icons and colours a Game gives the marks on the Map and the Compass.

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
}
