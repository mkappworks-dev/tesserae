# A dev structure that makes both channels visible

Status: ready-for-agent
Blocked by: 05, 06

Without this, nothing in this step is visible in `cargo dev`. `worldgen` is a
heightmap: no caves, no overhangs, so every visible face has an open column
above it and sky light is uniformly 15. The flood-fill is only observable where
something encloses space.

## What to build

`server/lightbox.rs`, `#[cfg(feature = "dev")]`, registered from
`server::plugin`. A system on `FixedUpdate` that waits until the Chunks it
needs are loaded, then once:

- Places a small roof on pillars on the surface near the spawn point, through
  `ChunkStore`'s Server-private `set_voxel`, and calls `relight` for each
  edited Voxel. Placing a solid Voxel where sky light was 15 is a **decrease**,
  so this exercises the de-propagation path live.
- Places a `LANTERN` Voxel under the roof. An **increase** from a single
  emitter: the torch case DESIGN.md names.

Both Voxel positions are derived in metres from `SpawnPoint` and converted
once, so the structure is the same physical size at any Voxel size (ADR-0003).

## Why the Server and not the Client

It mutates World state, so it cannot live on the Client side of ADR-0002 at
any price. It is not an edit *API* either - that is build-order step 5. This is
a `dev`-only Server system in the same family as `streaming::stats`, and it
exists so a screenshot can show the feature working.

## Tests

- The structure lands once and not every tick.
- After it lands, the Voxels under the roof have sky light below 15 and block
  light above 0, and the open ground beside it still has sky 15.
