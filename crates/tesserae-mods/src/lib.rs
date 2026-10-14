//! Loads and runs Mods: the WIT contract, the WASM sandbox and the Command queue.
//!
//! Exposes no `wasmtime` type, so the engine can be built without learning its
//! API and the wasmtime version stays one line in `Cargo.toml`. Depends on
//! neither Bevy nor wgpu. `docs/adr/0001-wasm-mod-boundary.md`.

mod bindings;
mod convert;
mod host;

#[cfg(test)]
mod tests;
