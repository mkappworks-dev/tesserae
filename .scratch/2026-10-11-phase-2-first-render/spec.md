# Phase 2: first render

Status: done

## Problem

Step 1 of the build order in [`DESIGN.md`](../../DESIGN.md) is finished:
`tesserae-core` stores palette-compressed Chunks and greedy-meshes them, with
27 tests and a criterion bench. None of it has ever been seen. A mesher is the
one subsystem where a plausible bug — a flipped winding, a transposed axis, a
UV that stretches instead of tiling — passes every unit test and is obvious the
instant a human looks at it.

Step 2 exists to look at it.

## Why now

It is the cheapest moment to discover that the vertex format is wrong. Steps 3
to 9 all build on `ChunkMesh`: streaming feeds it, light propagation adds a
vertex attribute to it, LOD re-meshes it, the look-pack API re-shades it.
Changing the format after streaming and lighting exist means touching all of
them.

Step 2 is also where the `tesserae` crate acquires Bevy, so it is where the
crate boundary that protects `tesserae-core` is first load-bearing rather than
theoretical.

## What a good outcome looks like

`cargo run -p tesserae` opens a window showing nine Chunks of lit terrain that
a human can fly around, and the meshing, scaling and shading are visibly
correct: no seams between Chunks, no inside-out faces, no stretched textures,
materials distinguishable from one another.

## Scope

In scope:

- `crates/tesserae/` as a Bevy 0.20 binary crate, in the workspace `members`.
- `ChunkMesh` -> Bevy `Mesh`, including a custom per-vertex Material attribute.
- A `MaterialExtension` over `StandardMaterial` that picks a flat colour per
  Material index.
- A 3x3 grid of `testgen::terrain_chunk(cx, 0, cz)` as `Mesh3d` entities.
- A fly camera and a directional light.
- Fixing CI's `Boundaries` job, which currently fails on every branch.

Out of scope:

- Real world generation and Chunk streaming. Step 3. `testgen` is what step 2
  renders, and the 3x3 grid is a fixed scene, not a streamer.
- The Server. Step 3 introduces the tick loop and the authority split. Step 2
  must not build a structure that assumes the Client owns the World, but it
  does not build the Server either.
- Texturing and the look-pack API. Step 9. Flat colours now; a tiny palette is
  enough to tell Stone from Dirt from Grass.
- Shaped Voxels. Step 5. `ChunkMesh::skipped_shaped` is already counted and
  stays unread.
- Light propagation. Step 4. `bevy_pbr`'s directional light and shadow maps are
  all the lighting step 2 has.

## Constraints that a change can break while looking correct

1. **`tesserae-core` and `tesserae-mods` acquire no `bevy` or `wgpu`
   dependency, even transitively.** The `ChunkMesh` -> `Mesh` conversion
   therefore lives in `crates/tesserae`. Invariant 3 in
   [`docs/architecture.md`](../../docs/architecture.md).
2. **Everything physical is in metres.** `voxel_size` is the single conversion
   from Voxel-index space, and `ChunkMesh` positions arrive in Voxel-index
   space. Camera speed is metres per second. Nothing assumes one Voxel is one
   metre. [ADR-0003](../../docs/adr/0003-voxel-size-is-per-world.md).
3. **The Server is the sole authority over World state.**
   [ADR-0002](../../docs/adr/0002-client-server-from-day-one.md).

## Known traps

- Bevy 0.20 is three days old and moved a great deal: bloom/DOF/motion-blur to
  `bevy_post_process`, anti-aliasing to `bevy_anti_alias`, light types to
  `bevy_light`, cameras to `bevy_camera`. Shaders are **WESL** (`.wesl`), not
  WGSL. The render graph was deleted in 0.19 and replaced by systems. Recalled
  snippets and most of the web describe the removed API; read the crate
  sources.
- The mesher's baseline on an M-series machine is terrain 358 us/chunk, solid
  337 us, checkerboard 2.39 ms, worldgen 117 us, release profile. Step 2 must
  not regress them. `cargo bench -p tesserae-core`.
- The first build takes 5-15 minutes: Bevy has never been compiled here.

## Done when

- `cargo run -p tesserae` shows nine Chunks of lit terrain, flyable.
- `cargo test --workspace` and
  `cargo clippy --workspace --all-targets --all-features` are clean.
- `cargo fmt --all --check` and `npm run fmt:check` pass.
- `cargo bench -p tesserae-core` shows no regression.
- CI's `Boundaries` job passes.

## Comments
