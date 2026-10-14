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

/// A name's id is its index. Plain data: step 7 writes no file, so nothing here
/// is serialised yet.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Registry {
    pub materials: Vec<String>,
    pub shapes: Vec<String>,
}

const BUILTIN_MATERIALS: [&str; 8] = [
    "core:air",
    "core:stone",
    "core:dirt",
    "core:grass",
    "core:lantern",
    "core:glass",
    "core:leaves",
    "core:water",
];
const BUILTIN_SHAPES: [&str; 4] = ["core:cube", "core:slab", "core:stair", "core:post"];

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
        let own = |names: &[&str]| names.iter().map(|&n| n.to_owned()).collect();
        Registry {
            materials: own(&BUILTIN_MATERIALS),
            shapes: own(&BUILTIN_SHAPES),
        }
    }

    /// How many names of `kind` are built in, which is where a Mod's ids start.
    pub fn builtin_len(kind: Kind) -> usize {
        match kind {
            Kind::Material => BUILTIN_MATERIALS.len(),
            Kind::Shape => BUILTIN_SHAPES.len(),
        }
    }

    pub fn names(&self, kind: Kind) -> &[String] {
        match kind {
            Kind::Material => &self.materials,
            Kind::Shape => &self.shapes,
        }
    }

    pub fn id_of(&self, kind: Kind, name: &str) -> Option<usize> {
        self.names(kind).iter().position(|n| n == name)
    }

    /// `saved` if every name in it is provided by this load, so a World loads
    /// its ids in the order it was saved in whatever order the Mods loaded.
    /// Names this load has and `saved` lacks are not in the result.
    pub fn replay(&self, saved: &Registry) -> Result<Registry, RegistryMismatch> {
        for kind in [Kind::Material, Kind::Shape] {
            let builtin = Registry::builtin();
            let builtin = builtin.names(kind);
            let saved_names = saved.names(kind);
            if let Some(id) = (0..builtin.len()).find(|&i| saved_names.get(i) != builtin.get(i)) {
                return Err(RegistryMismatch::BuiltinMoved { kind, id });
            }
            if let Some(name) = saved_names.iter().find(|n| self.id_of(kind, n).is_none()) {
                return Err(RegistryMismatch::Missing {
                    kind,
                    name: name.clone(),
                });
            }
        }
        Ok(saved.clone())
    }
}
