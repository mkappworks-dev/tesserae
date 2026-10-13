# Coarse meshing and delivery

Status: done
Blocked by: 03

Spec decisions 3, 5 (mesh half) and 7.

## What to build

- Mesh job for a coarse Chunk: `Arc<Chunk>` for it and its six same-level
  neighbours, a `ChunkLight::filled(Light::new(MAX_LIGHT, 0))` for all seven,
  the `Tables`; returns a `ChunkMesh` through the existing `mesh_chunk`. No
  change to `mesh.rs`, `light.rs` or `chunk.rs`.
- Dispatch only for Chunks in the level's mesh shell (outer minus hole), nearest
  first, requiring all six neighbours held. No settled-light gate (there is no
  light).
- Landing: delivered if inside `outer.grown(hysteresis)`; a result whose Chunk
  was dirtied during the flight is delivered anyway and stays dirty. Retire
  meshes that leave the keep shell (outside `outer.grown(h)` or inside the hole
  shrunk horizontally by `h`).
- New messages `LodMeshReady { level, coord, mesh }` and
  `LodUnloaded { level, coord }` in `server/streaming.rs`, written only by the
  tick. Level 0's messages are unchanged.
- `StreamingStats` (dev) gains per-level held / meshed counts for the capture
  report.

## Tests

- At quiescence the delivered set per level is exactly the mesh shell's Chunks
  that have six neighbours, and each delivered mesh equals a fresh `mesh_chunk`
  of the store with the synthetic light.
- No mesh is delivered before its six neighbours are held.
- Every vertex light on a coarse mesh is `Light::new(15, 0)`.
- Eviction safety, the module-doc argument per level: teleport the anchor with
  Jobs outstanding; no mesh lands whose neighbour was evicted.
- The hole: Chunks inside it are not meshed, Chunks inside the previous level's
  extent but outside the hole are (the overlap), and walking onto a coarse Chunk
  releases it only past the hysteresis.
- A level-1 mesh contains no shaped-Voxel geometry (`shaped_voxels == 0`).

## Acceptance criteria

- `./scripts/cargo-ci` passes.

## Comments

Built as specified except where listed. The mesh stage lives in `server/lod.rs`
beside generation (`mesh_coarse`, `land_meshes`, `dispatch_meshes`; `retire`
now writes `LodUnloaded`). 11 tests added there and one in `streaming.rs`
(`stats_count_what_each_level_holds_and_has_meshed`, dev); two ticket-03 tests
were rewritten because dirty marks are now consumed by meshing (the
regeneration test now asserts the Chunk and its six neighbours are each
re-delivered once, the first-generation test asserts a margin arrival delivers
nothing). Mutation checks, each restored: a missing neighbour replaced by the
Chunk itself fails the quiescence, six-neighbour and fly-by tests; dispatching
the whole outer extent instead of the shell fails the hole and the hysteresis
tests; pruning `delivered` before writing `LodUnloaded` fails the teleport
test; not re-dirtying a cancelled mesh job fails the come-back test.

Where the spec was wrong or silent:

- **Cancelled mesh jobs re-dirty their Chunk.** Dispatch takes a Chunk out of
  `dirty`, and `retire` drops jobs outside `keep` while the Chunk stays held
  (`keep` is inside `evict`), so nothing would ever mesh it again when the Eye
  returns. `retire` collects the cancelled keys first (`Stage::keys` is no
  longer test-only) and marks the ones still inside `evict` dirty. Level 0's
  `retire` looks to have the same hole; I did not touch it.
- **Landing checks `keep` (outer grown, minus the shrunk hole), not just
  `outer.grown(h)`.** Identical in practice, since `retire` cancels in-flight
  jobs for the hole first, but it is the one rule for what the Client may hold.
  A mesh can therefore be delivered for a Chunk outside the mesh shell but
  inside `keep` (a job in flight when the Eye moved a Cell); the Client keeps it
  until it leaves `keep`, as the spec's hysteresis wants.
- **A delivered Chunk outside the mesh shell is not re-meshed** when a
  neighbour is regenerated, because dispatch scans the shell only. Same as
  level 0. Ticket 05's edit only reaches the level-1 cover near the Eye, which
  is in the shell, so it does not bite there.
- **Gating:** a second flag, `mesh_pending`, beside ticket 03's `pending`. Set
  by an anchor move, any generate landing (used or dropped), any mesh landing,
  and a scan the budget cut short; `invalidate` sets only `pending`, because the
  regeneration's landing sets both. Offsets are cached per `Shell` now (the
  generate scan uses `Shell { outer: load, hole: None }`), so the mesh shell's
  list is shared by the levels with the same shell.
- **`OPEN_SKY` is a `static ChunkLight`**, as `SKY_CHUNK` is for level 0, so a
  job allocates nothing for light. The job is `mesh_coarse(&Chunk, &[Arc<Chunk>;
  6], &Tables)`; it does not go through `build_mesh`, which wants a light per
  Chunk.
- `get` and `coords` on `LodStore` are `cfg(test)` rather than allowed dead:
  nothing outside tests reads them. `shared` lost its allow. `LodStore::len`
  and `LodState::delivered_len` are dev-only, for the stats. The module-level
  `allow(dead_code)` in `lod_rings.rs` is gone; every item has a caller.
- **Stats:** `StreamingStats` lost `Copy` and gained `levels: Vec<LevelStats>`
  (`held`, `meshed`), indexed by level with level 0 repeating `loaded` and
  `meshed`. The capture prints it as a separate `capture_lod
  held/meshed=0:a/b,1:c/d,...` line, which `scripts/frame-runs` does not match,
  so its aggregation is unchanged. That one `info!` is the only Client file
  touched.
- `LodMeshReady` and `LodUnloaded` carry `cfg_attr(not(test), allow(dead_code))`
  until ticket 06 reads them. Remove it there.
- Test settings: the four-level harness again (mesh shell 5 columns x 3 = 15
  Chunks a level, no hole, since a one-Cell view has none). The hole tests use
  `hole_settings()`: view 192 m, vertical 64 m, hysteresis 32 m, reach 384 m,
  which gives level 1 a mesh hole of (2, 0), a keep hole of (1, 0) and 1773 held
  Chunks; it is the smallest setting whose keep hole is not `None`.

Sizes seen, for ticket 07: with `hole_settings()`, level 1 delivers 552 meshes,
213 of them non-empty, 128,596 quads in all (about 600 a non-empty Chunk) and
514,384 vertices, which at 40 bytes a vertex (position, normal, uv, material,
light) is about 20 MB for that ring. The four-level small harness meshes 9, 10
and 10 non-empty Chunks of 15 on levels 1 to 3 at 3,757, 5,630 and 5,723 quads.
These are test worlds, not the defaults; the real per-level table is ticket 07.

For ticket 05:

- A regeneration landing dirties the Chunk and its held face neighbours and sets
  both flags, so `LodState::invalidate(1, cover)` alone is enough to get the
  Chunk and its neighbours re-meshed and re-delivered, once each. That is the
  regeneration test.
- Its allow on `invalidate` is still there.

For ticket 06 (the Client):

- `LodMeshReady { level, coord, mesh }`: `level >= 1` only. `mesh` is a
  `tesserae_core::ChunkMesh` in that level's Voxel-index space (no Bevy, no
  scale applied). Scale it by `voxel_size * 2^level`; place the entity at
  `coord * cell_size * 2^level`, where `coord` is in the level's own lattice.
  Empty means despawn, as at level 0. A Chunk can arrive again after a
  regeneration (it replaces the entity's mesh) and after being unloaded.
- `LodUnloaded { level, coord }` is written when a mesh leaves `keep`: past the
  hysteresis outside, or inside the hole shrunk by it. Both are written in
  `FixedUpdate` by `stream_lod`, after `stream`.
- Every vertex's light attribute is `Light::new(15, 0)` packed, always. Every
  coarse mesh has `shaped_voxels == 0`. Both layers (`opaque`, `blended`) are
  filled; the Client has no blended draw yet.
- The Client sees a level's meshes overlap level-below ones by design (the mesh
  shell stops one Cell inside the finer level's extent); a Chunk delivered can
  also sit just outside its level's mesh shell for a while (see above).
