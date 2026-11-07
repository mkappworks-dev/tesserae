//! Loads and runs Mods: the WIT contract, the WASM sandbox and the Command queue.
//!
//! Exposes no `wasmtime` type, so the engine can be built without learning its
//! API and the wasmtime version stays one line in `Cargo.toml`. Depends on
//! neither Bevy nor wgpu. `docs/adr/0001-wasm-mod-boundary.md`.

mod adapt;
mod artwork;
mod bindings;
mod character;
mod command;
mod context;
mod convert;
mod data;
mod declare;
mod error;
mod host;
mod limits;
mod loader;
mod log;
mod manifest;
mod package;
mod prompt;
mod random;
mod recording;
mod registry;
mod sandbox;
mod sound;
mod trust;

pub use adapt::Line;
pub use bindings::exports::tesserae::mods::guest::{Registered, RegisteredId, WorldInfo};
pub use bindings::tesserae::mods::content::{Declaration, MovementPolicy};
pub use character::{
    Bone, Character, Cube, MAX_BONES, MAX_CUBES, Motion, Role, SKIN_BYTES, SKIN_EDGE,
};
pub use command::{
    BatchError, BatchReport, BoxCursor, Command, CommandBatch, Intent, MAX_FILL_VOXELS,
    MAX_VOXEL_INDEX, MAX_VOXELS_PER_CALL, Placement, Region, RegionError, Resolved, VoxelBox,
    validate, voxel_index,
};
pub use context::{EditEvent, EditKind, TickContext};
pub use declare::{
    Applied, Behaviour, DEFAULT_REMOVE_TIME, ITEM_SIZE_M, ItemDef, MATERIAL_TEXTURE_BYTES,
    MATERIAL_TEXTURE_EDGE, MAX_BEHAVIOURS_PER_MOD, MAX_BIOMES_PER_MOD, MAX_ITEMS,
    MAX_ITEMS_PER_MOD, MAX_MATERIALS, MAX_NAME_BYTES, MAX_ORES_PER_MOD, MAX_PER_MOD, MAX_SHAPES,
    MAX_STACK, MAX_TOOLS_PER_MOD, ToolDef, ToolKind, ViewParams, apply, resolve, resolve_biomes,
    resolve_items,
};
pub use error::ModError;
pub use limits::{MAX_COMMANDS, ManifestLimits, ModLimits};
pub use loader::{Compiled, ModHost};
pub use log::{LogDrain, LogHandle, LogLevel, LogLine};
pub use manifest::{
    DEFAULT_GAME, Found, MAX_SCAN_ENTRIES, Manifest, Migration, Rejected, SUPPORTED_API, Scan,
    games_dir, migrate_legacy_games, plan, scan, with_dependencies,
};
pub use package::{Staged, stage};
pub use prompt::{confirm_trust, trust_prompt};
pub use registry::{Kind, Registry, RegistryMismatch};
pub use sandbox::{ModFault, ModInstance};
pub use sound::{Sound, Sounds, resolve_sounds};
pub use trust::{CheckedPayload, Trust, Untrusted, Verdict};

#[cfg(test)]
mod tests;

/// Platform user data directory, without creating it or the Mods directory.
pub fn default_mods_dir() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|dir| dir.join("tesserae/mods"))
}
