use std::fmt;
use std::path::PathBuf;

use tesserae_core::{
    CombustionError, FluidError, MaterialError, ParamError, ReactionError, ShapeError,
};

use crate::registry::Kind;
use crate::trust::Untrusted;

/// Every variant names the Mod (or, before a Manifest parses, its directory).
#[derive(Debug)]
pub enum ModError {
    Io {
        who: String,
        path: PathBuf,
        source: std::io::Error,
    },
    Manifest {
        who: String,
        reason: String,
    },
    Api {
        id: String,
        api: String,
        supported: Vec<&'static str>,
    },
    DuplicateId {
        id: String,
        dirs: [PathBuf; 2],
    },
    SecondRoot {
        first: String,
        second: String,
    },
    UnknownDependency {
        id: String,
        depends: String,
    },
    Cycle {
        ids: Vec<String>,
    },
    Untrusted {
        id: String,
        reason: Untrusted,
    },
    Compile {
        id: String,
        source: wasmtime::Error,
    },
    Engine(wasmtime::Error),
    ReservedId {
        id: String,
    },
    NotAGame {
        id: String,
        field: &'static str,
    },
    TooMany {
        id: String,
        kind: Kind,
        count: usize,
        max: usize,
        scope: &'static str,
    },
    PaletteFull {
        id: String,
        count: usize,
        max: usize,
    },
    BadName {
        id: String,
        kind: Kind,
        row: usize,
        name: String,
        reason: &'static str,
    },
    DuplicateName {
        id: String,
        kind: Kind,
        row: usize,
        name: String,
    },
    Params {
        id: String,
        what: &'static str,
        source: ParamError,
    },
    Material {
        id: String,
        row: usize,
        name: String,
        source: MaterialError,
    },
    Shape {
        id: String,
        row: usize,
        name: String,
        source: ShapeError,
    },
    /// A component whose exported contract line is not its Manifest's `api`.
    LineMismatch {
        id: String,
        api: String,
        exported: &'static str,
    },
    TooManyRows {
        id: String,
        what: &'static str,
        count: usize,
        max: usize,
    },
    /// A fluid, reaction or fire row names a Material no loaded Mod registered.
    UnknownName {
        id: String,
        what: &'static str,
        row: usize,
        name: String,
    },
    Fluid {
        id: String,
        row: usize,
        name: String,
        source: FluidError,
    },
    Reaction {
        id: String,
        row: usize,
        source: ReactionError,
    },
    Combustion {
        id: String,
        row: usize,
        name: String,
        source: CombustionError,
    },
}

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModError::Io { who, path, source } => {
                write!(f, "mod `{who}`: {}: {source}", path.display())
            }
            ModError::Manifest { who, reason } => write!(f, "mod `{who}`: mod.toml: {reason}"),
            ModError::Api { id, api, supported } => write!(
                f,
                "mod `{id}` was built against api {api}, but this build of Tesserae supports {}",
                supported.join(" and ")
            ),
            ModError::DuplicateId { id, dirs } => write!(
                f,
                "mod id `{id}` is declared by both {} and {}",
                dirs[0].display(),
                dirs[1].display()
            ),
            ModError::SecondRoot { first, second } => write!(
                f,
                "mods `{first}` and `{second}` are both Games (`root = true`); at most one is allowed"
            ),
            ModError::UnknownDependency { id, depends } => {
                write!(
                    f,
                    "mod `{id}` depends on `{depends}`, which is not installed"
                )
            }
            ModError::Cycle { ids } => {
                write!(
                    f,
                    "mods depend on each other in a cycle: {}",
                    ids.join(", ")
                )
            }
            ModError::Untrusted { id, reason } => match reason {
                Untrusted::New => write!(f, "mod `{id}` is not trusted"),
                Untrusted::Changed => write!(f, "mod `{id}` changed since it was trusted"),
            },
            ModError::Compile { id, source } => write!(f, "mod `{id}`: cannot compile: {source}"),
            ModError::Engine(source) => write!(f, "cannot build the WASM engine: {source}"),
            ModError::ReservedId { id } => {
                write!(f, "mod id `{id}` is reserved for the engine's built-ins")
            }
            ModError::NotAGame { id, field } => write!(
                f,
                "mod `{id}` sets `{field}`, but only the Game (`root = true`) may"
            ),
            ModError::TooMany {
                id,
                kind,
                count,
                max,
                scope,
            } => write!(
                f,
                "mod `{id}` brings the {kind}s to {count}, but at most {max} are allowed {scope}"
            ),
            ModError::PaletteFull { id, count, max } => write!(
                f,
                "mod `{id}` brings the Materials to {count}, but the palette holds {max} beyond the built-ins"
            ),
            ModError::BadName {
                id,
                kind,
                row,
                name,
                reason,
            } => write!(
                f,
                "mod `{id}`: {kind} {row} is named `{name}`, which {reason}"
            ),
            ModError::DuplicateName {
                id,
                kind,
                row,
                name,
            } => write!(
                f,
                "mod `{id}`: {kind} {row} is named `{name}`, which is already registered"
            ),
            ModError::Params { id, what, source } => {
                write!(f, "mod `{id}`: {what}: {source}")
            }
            ModError::Material {
                id,
                row,
                name,
                source,
            } => write!(f, "mod `{id}`: Material {row} `{name}`: {source}"),
            ModError::Shape {
                id,
                row,
                name,
                source,
            } => write!(f, "mod `{id}`: Shape {row} `{name}`: {source}"),
            ModError::LineMismatch { id, api, exported } => write!(
                f,
                "mod `{id}`: its Manifest says api {api}, but its component is built against {exported}"
            ),
            ModError::TooManyRows {
                id,
                what,
                count,
                max,
            } => write!(
                f,
                "mod `{id}` declares {count} {what} rows, but at most {max} are allowed"
            ),
            ModError::UnknownName {
                id,
                what,
                row,
                name,
            } => write!(
                f,
                "mod `{id}`: {what} {row} names Material `{name}`, which no loaded Mod registers"
            ),
            ModError::Fluid {
                id,
                row,
                name,
                source,
            } => write!(f, "mod `{id}`: fluid {row} `{name}`: {source}"),
            ModError::Reaction { id, row, source } => {
                write!(f, "mod `{id}`: reaction {row}: {source}")
            }
            ModError::Combustion {
                id,
                row,
                name,
                source,
            } => write!(f, "mod `{id}`: combustion {row} `{name}`: {source}"),
        }
    }
}

impl std::error::Error for ModError {}
