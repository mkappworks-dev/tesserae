# Worldgen and Chunk streaming, behind the client/server split

Status: done

Build-order step 3 in [`DESIGN.md`](../../DESIGN.md): "Worldgen + chunk
streaming, behind the client/server split with the server in-process."

## Problem

Step 2 put a fixed 3x3 grid of Chunks on screen and never mutated anything.
Three things are therefore still theoretical.

**There is no World generation.** `tesserae_core::testgen` is a bench fixture
whose noise is tuned in Voxel units: `1.0 / 48.0` per Voxel index, a sea level
of 12 and a relief of 18, all dimensionless. ADR-0003 calls tuning a noise
frequency in Voxel units the one decision that cannot be repaired later, and
the fixture gets away with it only because its job is to feed the mesher a
surface with overhangs, not to make a World. Its own module doc says so.

**The client/server split is written down and untested.** ADR-0002 says the
Server is the sole authority permitted to mutate a Chunk, and step 2 has no
writer of World state at all, so nothing has had to obey it yet. The invariant
only starts costing something once there is something to mutate, which is this
step.

**Nothing streams.** `crates/tesserae/src/scene.rs` spawns nine Chunks at
`Startup` and its module doc says step 3 deletes it. Per-Chunk cost has not
mattered so far; with streaming it becomes the frame budget, and DESIGN.md's
per-ring Chunk budget is still "a number to measure, not to guess".

## What a good outcome looks like

`cargo dev` opens a window on generated terrain that extends to the configured
view distance, and flying the camera streams Chunks in ahead and out behind
with no visible seams and no frame hitch. Every Chunk that exists was written
by a Server system. The Client has no path to World state at all, enforced by
module visibility rather than by a comment.

## Scope

### In scope

- **`tesserae-core`: a real generator.** A `worldgen` module taking a seed,
  `voxel_size` and a parameter struct whose every spatial field is in metres.
  Deterministic, no Bevy, no new dependency. The value-noise primitive is
  shared with `testgen` rather than copied.
- **`tesserae-core`: a meshing entry point that can decline.** Streaming a 3D
  ring means most candidate Chunks are uniform air, and meshing one still costs
  a 34-cubed `Neighborhood` build plus a three-axis sweep. A single call that
  takes the centre Chunk and its six neighbours, answers "no faces" without
  building the padded copy where it provably can, and otherwise meshes.
- **`tesserae`: a Server.** `src/server/` with the Chunk store, the spawn
  point, and a tick on `FixedUpdate`. The store's mutating API is visible only
  inside `crate::server`.
- **`tesserae`: streaming.** Load and evict around an anchor the Client
  nominates, with a measured per-tick budget, hysteresis so a camera sitting on
  a Cell boundary does not thrash, and correct face culling at the edge of the
  loaded set.
- **`tesserae`: a Client that only receives.** Buffered messages carrying a
  finished `ChunkMesh` in, and a coordinate out. `scene.rs` is deleted and the
  camera starts from the Server's spawn point.
- **Numbers.** `cargo bench -p tesserae-core` before and after, in the PR, and
  the per-tick budget recorded as a measurement rather than a guess.

### Out of scope

- **LOD.** Step 6. The ring here is one level, full detail.
- **Light propagation.** Step 4. The flat ambient fill from step 2 stays.
- **Shaped Voxels.** Step 5. The generator emits full cubes only.
- **Mod-supplied worldgen parameters.** Step 7 owns the WIT contract. The
  parameter struct is shaped so a Mod can fill it — plain data, no callbacks —
  but nothing crosses a WASM boundary yet.
- **Saving and loading.** A Chunk evicted here is regenerated, not read back.
  Generation is deterministic, so the world is stable across an eviction; a
  save format is a later step and an edited Chunk has nowhere to go until then.
- **Off-thread generation and meshing.** A measured main-thread budget is
  enough at this step's view distance; see "Decisions" below.
- **Network transport.** Step 10.

## Decisions

These are the choices that shape the diff, with the reason each one is not the
obvious alternative.

### The Client never sees the Chunk store

The architecture diagram already routes meshing through the Server:
`STREAM -> MESHER -> UPLOAD`. Taking that literally, the Server generates and
meshes, and hands the Client a `ChunkMesh` — a `tesserae-core` type with no
Bevy in it — through a buffered message. The Client converts it to a Bevy
`Mesh` and owns the entity.

The alternative, a Client that reads the store and meshes for itself, needs the
store to be public, and then ADR-0002 is back to being a convention a reviewer
has to remember. Routing the mesh through a message lets `ChunkStore`'s
mutators be `pub(in crate::server)`, so a Client system that tries to write
World state does not compile. That is the point of doing the split now.

### The anchor is a Client component, read by the Server

The Server streams around whatever entity carries a `StreamingAnchor`
component, which the Client puts on its camera. The Server reads that
entity's `Transform`.

This is intent, not state: the camera is a Client entity, its pose is Client
data, and the Server only reads it. Defining the component in `server/` rather
than `client/` says which side declares the contract — the Server publishes the
intent it accepts.

### The Server ticks on `FixedUpdate`

A tick loop at a fixed rate is what the network transport in step 10 will need,
and `RunFixedMainLoop` runs before `Update`, so a message the Server writes
reaches the Client in the same frame. The reverse direction can drop messages
when a frame runs no fixed tick, which is a second reason the Client only ever
receives.

### Three radii, not one

Face culling reads one Voxel across each face, so a Chunk meshed before its six
neighbours exist emits a skin of faces buried in the neighbour's rock. The
loaded set is therefore the meshed set dilated by one Cell, and a Chunk is
meshed only once all six neighbours are present. Arriving neighbours mark their
neighbours for re-meshing.

Eviction trails loading by a further Cell plus hysteresis. That ordering is
load-bearing: an evicted Chunk is then never adjacent to a live mesh, so
eviction cannot leave a stale face behind and needs no re-mesh of its own.

### Main-thread generation and meshing, with a budget

Step 2's measured cost is ~360 microseconds to mesh a terrain Chunk and ~114 to
generate one. At this step's default view distance the steady-state inflow when
flying at the camera's top speed is a slab of Cells every half second, which a
small per-tick budget absorbs inside a tick at 64 Hz.

Off-thread generation via `AsyncComputeTaskPool` is the eventual answer and is
deliberately deferred: it adds a snapshot-versus-live question about what a task
may read, and the budget has to be measured either way. Measure first, move it
off the main thread when the measurement says to.

### The generator's cheap answers

Fractal noise normalised to `[0, 1]` bounds the surface between `sea_level_m`
and `sea_level_m + relief_m`. A Chunk wholly above that band is uniform air and
a Chunk wholly below it, by more than the soil depth, is uniform stone — both
answerable without sampling noise at all. In a 3D ring most Chunks are one or
the other, so this is not a micro-optimisation but the reason a vertical extent
is affordable.

## Invariant checks for the reviewer

Each of these is a mistake that will not look wrong in review.

1. **No Client system mutates World state.** Check that `ChunkStore`'s
   mutators are not reachable from `crate::client`, and that the Client's only
   inputs are the two message types. ADR-0002.
2. **`tesserae-core` acquires no `bevy` or `wgpu` edge.** The generator and the
   mesher entry point take and return plain data. CI's `Boundaries` job.
3. **Every spatial number in the generator is in metres.** Grep the `worldgen`
   module for a bare Voxel count. The test that pins this generates the same
   World at two Voxel sizes and asserts the surface lands at the same metre
   height. ADR-0003.
4. **View distance is not World save data.** `WorldSettings` owns `voxel_size`
   and `seed`, because CONTEXT.md says a World owns its Voxel size, its seed
   and its Chunks. How far the Server streams is a runtime setting and lives
   with the Server.

## Done when

- `cargo dev` streams Chunks in and out around a moving camera, with every
  write to World state performed by a Server system.
- `cargo test --workspace` and `cargo lint` are clean.
- `cargo fmt --all --check` and `npm run fmt:check` pass.
- `cargo deny check advisories licenses bans sources` passes.
- `cargo bench -p tesserae-core` shows no regression against step 2's terrain
  ~360 us, solid ~339 us, checkerboard ~2.46 ms, worldgen ~114 us.
- `scene.rs` is gone.
- A screenshot is in the PR.

## Comments
