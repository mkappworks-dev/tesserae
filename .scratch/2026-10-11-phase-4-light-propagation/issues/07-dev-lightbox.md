# A dev structure that makes both channels visible

Status: done
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

## Comments

Lives in `server/lightbox.rs` and orders itself through `FixedPostUpdate` rather
than `.after(streaming::stream)`, which would have meant making a private system
and a private resource type public for a `dev` feature's benefit.

Two revisions came from actually looking at the screenshot. A roof on four
corner pillars showed nothing: the camera starts 20 m above the spawn point, so
it saw the slab's top. And at 5 m of clearance the interior was foreshortened
into a two-Voxel sliver. It is now a 12 m tall, 18 m deep shelter walled on
three sides with the fourth facing the camera.

Its edits are budgeted per tick. Placing ~700 Voxels in one frame cost about
33 ms, which is this tool building a building and not what an edit costs — a
single edit is bounded by the 15 Voxels light reaches.
