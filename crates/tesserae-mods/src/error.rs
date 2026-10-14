use std::fmt;
use std::path::PathBuf;

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
        }
    }
}

impl std::error::Error for ModError {}
