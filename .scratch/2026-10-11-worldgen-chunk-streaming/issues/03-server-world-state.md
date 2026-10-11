# The Server: World state, the tick, and the spawn point

Status: ready-for-agent
Blocked by: 01

ADR-0002 says the Server is the sole authority permitted to mutate a Chunk, and
step 2 has no writer of World state at all, so nothing has had to obey it. This
ticket creates the authority and makes the invariant a visibility fact rather
than a convention.

## Task

Add `crates/tesserae/src/server/` with `mod.rs` and `world_state.rs`, and
register `server::plugin` in `main.rs`.

### `world_state.rs`

- `ChunkStore`, a `Resource` wrapping the loaded Chunks by `ChunkCoord`.
  Read accessors are `pub`. **Every mutator is `pub(in crate::server)`** —
  insert, remove, and anything returning `&mut Chunk`. A Client system can
  still name `ResMut<ChunkStore>`; with the mutators out of scope it cannot do
  anything with it, and that is the enforcement. Say so in a comment, with the
  ADR link, because the visibility looks arbitrary otherwise.
- `SpawnPoint`, a `Resource` holding metres, with a private field, a `pub`
  getter, and a `pub(in crate::server)` constructor. Same reason.

### `mod.rs`

- `pub fn plugin(app: &mut App)`.
- Inserts a `Worldgen` resource built from `WorldSettings`' `voxel_size` and
  `seed` plus `WorldgenParams::default()`.
- A `Startup` system that computes `SpawnPoint` by asking the generator for the
  surface height in metres at the world origin and adding a clearance in
  metres. Not in Voxels: a clearance of "two Voxels" is calibrated to one Voxel
  size. ADR-0003.
- The tick, on `FixedUpdate`. Ticket 04 fills it; here it is the ordered set
  the streaming systems join.

### `WorldSettings`

Add `seed: u64` in `crates/tesserae/src/world.rs`. CONTEXT.md: a World owns its
Voxel size, its seed and its Chunks. Nothing else goes in — how far the Server
streams is a runtime setting and belongs with the Server, not in save data.

## Tests

- `ChunkStore` round-trips a Chunk through its own module's mutators.
- `SpawnPoint` sits above the generated surface at the origin, and does so at
  two different Voxel sizes — the clearance is in metres, so the metre height
  should barely move when `voxel_size` halves.
- A unit test asserting the spawn point is above the surface, not merely above
  zero.

## Done when

`cargo test --workspace` and `cargo lint` are clean, `cargo dev` still opens a
window, and `grep -rn "pub(in crate::server)" crates/tesserae/src/server/`
covers every mutating method on the two resources.

## Comments
