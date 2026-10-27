//! The ordered names a World's `MaterialId`s and `ShapeId`s were handed out in.

use std::fmt;

/// Which of the two id spaces a name or a limit is about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Material,
    Shape,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Material => "Material",
            Kind::Shape => "Shape",
        })
    }
}

/// A name's id is its index; a retired id is a slot with no name, which keeps
/// the ids after it in place. Plain data: step 7 writes no file, so nothing
/// here is serialised yet.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Registry {
    pub(crate) materials: Vec<Option<String>>,
    pub(crate) shapes: Vec<Option<String>>,
}

/// Retired ids retain unnamed slots so Mod ids keep starting at 8.
const BUILTIN_MATERIALS: [Option<&str>; 8] =
    [Some("core:air"), None, None, None, None, None, None, None];
const BUILTIN_SHAPES: [Option<&str>; 4] = [
    Some("core:cube"),
    Some("core:slab"),
    Some("core:stair"),
    Some("core:post"),
];

/// A saved order that this load cannot honour.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RegistryMismatch {
    /// The save names something no loaded Mod provides.
    Missing { kind: Kind, name: String },
    /// A built-in is not at its permanent id in the save.
    BuiltinMoved { kind: Kind, id: usize },
}

impl fmt::Display for RegistryMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryMismatch::Missing { kind, name } => {
                write!(
                    f,
                    "the save uses {kind} `{name}`, which no loaded Mod provides"
                )
            }
            RegistryMismatch::BuiltinMoved { kind, id } => {
                write!(f, "the save does not have the built-in {kind} at id {id}")
            }
        }
    }
}

impl std::error::Error for RegistryMismatch {}

impl Registry {
    pub fn builtin() -> Self {
        let own = |names: &[Option<&str>]| names.iter().map(|n| n.map(str::to_owned)).collect();
        Registry {
            materials: own(&BUILTIN_MATERIALS),
            shapes: own(&BUILTIN_SHAPES),
        }
    }

    /// How many ids of `kind` are built in, a retired one included, which is
    /// where a Mod's ids start.
    pub fn builtin_len(kind: Kind) -> usize {
        match kind {
            Kind::Material => BUILTIN_MATERIALS.len(),
            Kind::Shape => BUILTIN_SHAPES.len(),
        }
    }

    fn slots(&self, kind: Kind) -> &[Option<String>] {
        match kind {
            Kind::Material => &self.materials,
            Kind::Shape => &self.shapes,
        }
    }

    /// Ids handed out so far, a retired one included: the next id.
    pub fn len(&self, kind: Kind) -> usize {
        self.slots(kind).len()
    }

    pub(crate) fn push(&mut self, kind: Kind, name: String) {
        match kind {
            Kind::Material => self.materials.push(Some(name)),
            Kind::Shape => self.shapes.push(Some(name)),
        }
    }

    /// Every registered name with its id, in id order. A retired id is not
    /// among them.
    pub fn names(&self, kind: Kind) -> impl Iterator<Item = (usize, &str)> {
        self.slots(kind)
            .iter()
            .enumerate()
            .filter_map(|(id, name)| Some((id, name.as_deref()?)))
    }

    pub fn id_of(&self, kind: Kind, name: &str) -> Option<usize> {
        self.names(kind).find(|&(_, n)| n == name).map(|(id, _)| id)
    }

    /// `saved` if every name in it is provided by this load, so a World loads
    /// its ids in the order it was saved in whatever order the Mods loaded.
    /// Names this load has and `saved` lacks are not in the result.
    pub fn replay(&self, saved: &Registry) -> Result<Registry, RegistryMismatch> {
        for kind in [Kind::Material, Kind::Shape] {
            let builtin = Registry::builtin();
            let builtin = builtin.slots(kind);
            let saved_slots = saved.slots(kind);
            if let Some(id) = (0..builtin.len()).find(|&i| saved_slots.get(i) != builtin.get(i)) {
                return Err(RegistryMismatch::BuiltinMoved { kind, id });
            }
            if let Some((_, name)) = saved
                .names(kind)
                .find(|&(_, n)| self.id_of(kind, n).is_none())
            {
                return Err(RegistryMismatch::Missing {
                    kind,
                    name: name.to_owned(),
                });
            }
        }
        Ok(saved.clone())
    }
}
