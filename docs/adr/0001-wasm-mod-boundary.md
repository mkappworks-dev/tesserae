# Mods run in WASM, and the host API is batched

Mods are WASM modules loaded by `wasmtime` against a versioned WIT contract, not
Lua (Luanti's choice, which caps performance — our differentiator) and not
native Rust dylibs (no stable Rust ABI, no sandbox, per-OS builds pushed onto
modders). Every WASM instance has its own linear memory, so crossing the
boundary copies: the cost that matters is **call frequency, not throughput**.

## Consequences

The host API is batched region operations and a command queue. A per-voxel
`set_block()` host call is the mistake this ADR exists to prevent — it is the
obvious, convenient design and it will not look wrong in review.

World generation, meshing, light propagation and fluid simulation never run in
WASM. They stay native in `tesserae-core`; mods parameterize them.
