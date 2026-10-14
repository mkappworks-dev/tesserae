//! Loads and runs Mods: the WIT contract, the WASM sandbox and the Command queue.
//!
//! Exposes no `wasmtime` type, so the engine can be built without learning its
//! API and the wasmtime version stays one line in `Cargo.toml`. Depends on
//! neither Bevy nor wgpu. `docs/adr/0001-wasm-mod-boundary.md`.

mod bindings;
mod convert;
mod declare;
mod error;
mod host;
mod limits;
mod loader;
mod manifest;
mod registry;
mod trust;

pub use bindings::tesserae::mods::content::Declaration;
pub use declare::{
    Applied, MAX_MATERIALS, MAX_NAME_BYTES, MAX_PER_MOD, MAX_SHAPES, ViewParams, apply,
};
pub use error::ModError;
pub use limits::{ManifestLimits, ModLimits};
pub use loader::{Compiled, ModHost};
pub use manifest::{Found, Manifest, SUPPORTED_API, discover, plan};
pub use registry::{Kind, Registry, RegistryMismatch};
pub use trust::{Trust, Untrusted, Verdict};

#[cfg(test)]
mod tests;
