# 01: Preserve cell state through one Server Edit path

Status: done
Blocked by: None

Spec: [decisions 2, 3, 5 and 11](../spec.md).

## What to build

Prefactor authoritative cell state so the packed solid Voxel can coexist
with fluid and combustion data without changing today's dry terrain and
generated-water behavior. Keep one canonical interpretation and one
Server-private World-change path for all callers.

This is the explicit prefactoring exception to vertical slices: it prepares
the shared storage/Edit interface before simulation starts changing cells.

## Acceptance criteria

- [x] Packed Voxel layout and permanent Material/Shape ids are unchanged.
  No Orientation or Shape bits encode fluid or burning state.
- [x] Owned native cell/Chunk state and immutable Snapshots carry the
  additional state without independently mutable copies of the same water.
- [x] Dry/unburning Chunks allocate no dense auxiliary arrays; uniform lake
  water retains compact storage. Record memory evidence in Comments.
- [x] Existing generation, player Edits, Mod Commands, and dev builders use
  the shared mutation rules. Validation, shaped caps, light, boundary meshes,
  and level-1 invalidation still work.
- [x] Auxiliary-state changes invalidate relevant Snapshot dependencies even
  when the solid Voxel is unchanged. Old Chunk lifetimes cannot be confused
  with new ones at the same coordinate.
- [x] Existing generated-water appearance, solid picking, collision, and
  0.1 Mod behavior remain covered through native and Server integration tests.
- [x] Capture pre-change native hot-path baselines for affected meshing,
  collision, and LOD comparisons, following the spec's measurement policy.
- [x] No Client mutator, core Bevy/wgpu edge, or dependency security downgrade
  is introduced.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- 2026-10-22: triaged ready-for-agent after the breakdown review. The only
  ticket with no blocker, so implementation starts here.
- 2026-10-22: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- `tesserae-core/src/occupants.rs`: `Occupants { solid, fluid, burning }` is
  the one reading of a position. `Occupants::of(voxel, overlay, fluids)`
  decodes the stored pair and `encode` is its canonical inverse: a packed
  fluid cube (generation's lake water, a legacy placement) means "full source,
  no solid" and carries no overlay fluid; every other wet state (flowing,
  partial surface, waterlogged) lives in the overlay with its Material. So a
  Voxel's fluid Material has exactly one home and never two copies.
  `place_solid` is the whole-Voxel rule every `set_voxel` caller goes
  through: a fluid Material placed the old way is a source, air breaks a
  solid and keeps its fluid, air over a cell with no solid empties it as
  before, any solid displaces fluid (partial Shapes keep it once ticket 04
  gives them geometry), and any solid change ends burning.
- `Fluid { material, source, surface }` (surface in sixteenths of the edge,
  dimensionless) and `Burning { fuel }`. `Overlay { fluid, burning }`,
  `Overlay::NONE` for dry terrain. `ChunkOverlay` stores one per Voxel.
- `tesserae-core/src/chunk.rs`: the palette bit packing is now a `pub(crate)`
  generic `Palette<T>` shared by `Chunk` and `ChunkOverlay`; `Chunk`'s public
  API and the packed Voxel are unchanged, and no Shape or Orientation bit
  changed meaning.
- `tesserae-core/src/fluid.rs`: `FluidTable` keyed by `MaterialId`, with
  `FluidDef` in metres and metres per second (reach, sideways, downward,
  recession, reset drop) holding the spec's proposed water tuning, pending
  ticket 09. `register` refuses non-finite or non-positive values and a
  second definition for one Material; `Tables::register_fluid` also refuses
  an unknown or solid Material. `Tables.fluids` joins `shapes` and
  `materials`; `Tables::vet_occupants` is the one check every writer passes
  (vetted solid, no fluid Material in the solid slot, known fluid, no fluid
  under a full cube, surface in range, no fire without a solid).
- `server/world_state.rs`: `ChunkStore` holds `Arc<ChunkOverlay>` and a
  `Lifetime` per Chunk beside the Chunk and its light. `shared` returns a
  `Snapshot { lifetime, chunk, light, overlay }`; `unchanged_since` compares
  lifetime and both `Arc`s. `set_overlay` goes through `Arc::make_mut` like
  `set_voxel`. `insert` assigns a fresh lifetime, so an evicted and
  regenerated Chunk at one coordinate is a different one. `occupants(at,
  fluids)` is the Server's read through the one interpretation.
- `server/edit.rs`: `set_occupants` is the shared World-change path:
  normalise, `vet_occupants`, canonical `encode`, shaped cap on the packed
  Voxel, overlay write, relight when the packed Voxel changed, then the
  existing invalidation (own mesh, light-touched meshes, boundary neighbours,
  level-1 cover) whether or not the packed Voxel changed. `set_voxel` is
  `place_solid` then `set_occupants`, so players, the Mod drain, the `dev`
  builders and the gallery keep their call and gain the rules. New
  `RejectReason`s mirror the new `PlaceFault`s.
- `server/jobs.rs` and `streaming.rs`: a relight job holds a `Snapshot` and
  lands only if `unchanged_since`, so an overlay change under a running solve
  drops it as a Voxel change does. Mesh jobs hold `Snapshot`s too.
- Light still reads the packed Voxel alone: the overlay has no optical effect
  until the tickets that give fluid and fire one, so an overlay-only change
  does not relight (commented at the one place that decides it).

### Verification

- Core: 221 lib tests (212 before). New: canonical decode, encode round
  trip, `place_solid` rules, overlay storage and memory, fluid registration,
  `register_fluid`, `vet_occupants`.
- Server: 281 tests in the `tesserae` binary (273 before). New: overlay and
  lifetime on insert and remove, overlay write invisible to a held Snapshot,
  old-lifetime Snapshot stale after regeneration, store reads through the one
  interpretation, overlay-only change dirties the mesh, the boundary
  neighbour and the cover and stales the Snapshot, refused readings change
  nothing, `set_voxel` through the layer rules, and a running solve whose
  overlay changed is dropped at landing (mutation-checked: removing the
  overlay clause of `unchanged_since` fails it).
- Every pre-existing generation, Edit, Mod-drain, gallery, picking, collision
  and 0.1 guest test passes unchanged. Full `scripts/cargo-ci` passed on the
  tree this commit records (rustfmt, prettier, clippy, default build, guests,
  tests, bench compile, boundaries, cargo-deny).
- No Client mutator was added; every new mutator is `pub(in crate::server)`.
  `tesserae-core` gained no dependency. Wasmtime/WASI 49.0.4 and Cranelift
  0.136.4 are untouched (`Cargo.lock` unchanged).

### Memory evidence

Measured by `dry_and_uniform_lake_chunks_allocate_no_overlay`:

| Chunk                              | packed bytes | overlay bytes |
| ---------------------------------- | -----------: | ------------: |
| dry uniform (air or stone)         |            0 |             0 |
| uniform lake water                 |            0 |             0 |
| one wet or burning Voxel           |          n/a |          4112 |

`Overlay` is 8 bytes. One non-default Voxel promotes the overlay to a 1-bit
index plane (4096 bytes) plus two palette entries, the same step a Chunk's
palette takes; a Chunk's dense light is 32 KiB for comparison. Per held Chunk
the store adds one `Arc` and one `u64` lifetime. Nothing dense is allocated
for a Chunk that never held fluid or fire.

### Hot-path baselines

Pre-change bench binaries were built from this branch at `2ae7399` (the
planning commit, before any code change) and kept outside the tree;
post-change binaries from the implementation before review, whose later
edits to `tesserae-core` were to comments only. Both carry an uncommitted
criterion patch measuring thread CPU time (`CLOCK_THREAD_CPUTIME_ID`), the
harness `.scratch/2026-10-12-joint-coverage-light/issues/04-measure-and-docs.md`
describes, because wall-clock figures on this host are distorted by other
sessions' builds. The patch and runner scripts are in the measurement
directory (`/private/tmp/tesserae-p8-t01.*`, unique, not committed).

Five interleaved rounds (round 1 and 3 and 5 pre first, 2 and 4 post first),
each round running the meshing, collide and lod binaries of one side and then
the other, criterion `--warm-up-time 1 --measurement-time 2`. Each figure
below is the minimum or the median, over the five rounds, of criterion's
per-round median. `uptime` was logged before and after every process:

| round | wall time   | load across its six processes |
| ----- | ----------- | ----------------------------- |
| 1     | 14:25-14:37 | 2.46-14.47                    |
| 2     | 17:40-17:53 | 2.04-3.96                     |
| 3     | 17:53-18:05 | 3.36-4.25                     |
| 4     | 19:42-19:55 | 1.81-3.72                     |
| 5     | 19:55-20:07 | 1.67-2.51                     |

Discarded, not in the figures: a first round 2 cut off when the session
ended at 14:48, and a first round 4 during which the host idle-slept at 18:10
(`pmset -g log`; one lod run took 47 minutes against 2). Rounds 4 and 5 were
rerun under `caffeinate -i`. Round 1's first process started at load 14.5
and ended at 3.7; dropping round 1 moves no conclusion below.

**Result.** No hot path regressed beyond the run's noise. Leaving out
`light_chunk/open_sky`, every minimum moved between -4.7% and +2.3%. Five
rows rose by more than 1% on the minimum:

| bench                                 | min   | median |
| ------------------------------------- | ----- | ------ |
| `light_chunk/opaque` (3.4 to 3.5 ns)  | +2.3% | +2.4%  |
| `lod_coarse_light/above_band_level_1` | +1.9% | +2.2%  |
| `greedy_mesh/glass_room`              | +1.4% | -0.2%  |
| `light_chunk/leaves_canopy`           | +1.4% | +2.9%  |
| `lod_mesh_coarse_lit/surface_level_5` | +1.3% | +2.0%  |

The two nanosecond rows are a uniform early-out each, and
`light_chunk/open_sky` reads -22% (4.5 to 3.5 ns) for the same reason: code
placement, not work. `Chunk::get`, `Chunk::set`, `kinds` and `shaped_units`
now go through the generic `Palette<T>`; the meshing, collision, downsample
and worldgen rows that call them hardest moved between -1.7% and +1.4%, and
the LOD mesh rows between -3.5% and +1.3%.

Pre-change figures, the baseline ticket 09 compares against (pre is
`2ae7399`, post is this commit; same harness, same rounds):

| bench | pre min | pre med | post min | Δ min | Δ med |
| --- | ---: | ---: | ---: | ---: | ---: |
| collide/fall_cubes | 40.2 ns | 40.4 ns | 40.3 ns | +0.2% | +0.6% |
| collide/fall_slabs | 41.3 ns | 41.7 ns | 41.4 ns | +0.2% | +0.5% |
| collide/fall_terrain | 152.9 ns | 153.9 ns | 153.4 ns | +0.3% | +0.7% |
| collide/ray_cubes | 407.6 ns | 408.9 ns | 407.9 ns | +0.1% | +0.4% |
| collide/ray_slabs | 407.3 ns | 408.0 ns | 407.8 ns | +0.1% | +0.0% |
| collide/ray_terrain | 473.5 ns | 474.7 ns | 473.0 ns | -0.1% | -0.1% |
| collide/walk_into_wall_cubes | 45.0 ns | 45.0 ns | 45.1 ns | +0.2% | +2.0% |
| collide/walk_into_wall_slabs | 44.8 ns | 44.9 ns | 44.9 ns | +0.2% | +1.2% |
| greedy_mesh/all_stairs | 3.62 ms | 3.62 ms | 3.56 ms | -1.7% | -1.0% |
| greedy_mesh/checkerboard | 2.59 ms | 2.62 ms | 2.55 ms | -1.5% | -0.3% |
| greedy_mesh/flat_fences_at_cap | 546.3 us | 551.0 us | 548.2 us | +0.3% | +0.4% |
| greedy_mesh/glass_room | 258.1 us | 264.2 us | 261.8 us | +1.4% | -0.2% |
| greedy_mesh/solid | 338.9 us | 349.6 us | 338.1 us | -0.2% | -2.6% |
| greedy_mesh/terrain | 345.9 us | 349.6 us | 344.1 us | -0.5% | -0.6% |
| greedy_mesh/terrain_canopy | 409.0 us | 414.8 us | 411.9 us | +0.7% | +0.2% |
| greedy_mesh/terrain_fences_at_cap | 602.3 us | 604.9 us | 603.8 us | +0.3% | +0.6% |
| greedy_mesh/terrain_light_gradient | 364.4 us | 365.5 us | 364.1 us | -0.1% | +1.2% |
| greedy_mesh/terrain_stairs_at_budget | 483.0 us | 485.2 us | 484.3 us | +0.3% | +3.5% |
| greedy_mesh/terrain_stairs_at_cap | 605.3 us | 608.2 us | 605.7 us | +0.1% | +0.4% |
| greedy_mesh/terrain_water | 397.3 us | 401.1 us | 394.9 us | -0.6% | +0.1% |
| light_chunk/cave_dark_neighbours | 44.9 us | 45.0 us | 43.2 us | -3.9% | +1.7% |
| light_chunk/cave_mouth | 102.5 us | 103.3 us | 103.3 us | +0.8% | +2.2% |
| light_chunk/flooded_air | 119.0 us | 119.2 us | 118.2 us | -0.7% | +2.0% |
| light_chunk/leaves_canopy | 169.4 us | 170.9 us | 171.7 us | +1.4% | +2.9% |
| light_chunk/opaque | 3.4 ns | 3.4 ns | 3.5 ns | +2.3% | +2.4% |
| light_chunk/open_sky | 4.5 ns | 4.5 ns | 3.5 ns | -22.4% | -22.5% |
| light_chunk/ramp_densest | 199.3 us | 199.8 us | 197.3 us | -1.0% | +3.0% |
| light_chunk/straddling | 182.1 us | 183.5 us | 179.2 us | -1.6% | -0.2% |
| light_chunk/straddling_settled | 297.2 us | 297.4 us | 296.0 us | -0.4% | +1.5% |
| light_chunk/straddling_shaped | 156.3 us | 157.0 us | 153.2 us | -2.0% | -0.1% |
| light_chunk/straddling_unsmoothed | 89.0 us | 89.1 us | 87.8 us | -1.4% | +1.7% |
| light_chunk/straddling_water | 132.5 us | 132.7 us | 132.1 us | -0.3% | +1.1% |
| lod_coarse_chunk/air_level_1 | 50.6 ns | 50.9 ns | 50.8 ns | +0.3% | -0.3% |
| lod_coarse_chunk/rock_level_1 | 69.2 ns | 69.3 ns | 69.2 ns | +0.1% | +0.2% |
| lod_coarse_chunk/surface_level_1 | 2.11 ms | 2.12 ms | 2.11 ms | -0.0% | +0.4% |
| lod_coarse_chunk/surface_level_2 | 1.29 ms | 1.29 ms | 1.29 ms | +0.1% | +0.2% |
| lod_coarse_chunk/surface_level_3 | 1.42 ms | 1.42 ms | 1.40 ms | -1.2% | -0.8% |
| lod_coarse_chunk/surface_level_4 | 957.4 us | 985.3 us | 948.6 us | -0.9% | -3.1% |
| lod_coarse_chunk/surface_level_5 | 773.0 us | 775.1 us | 770.0 us | -0.4% | -0.2% |
| lod_coarse_light/above_band_level_1 | 17.8 ns | 17.8 ns | 18.1 ns | +1.9% | +2.2% |
| lod_coarse_light/surface_level_1 | 61.7 us | 61.9 us | 62.2 us | +0.8% | +0.7% |
| lod_coarse_light/surface_level_2 | 86.8 us | 87.1 us | 87.2 us | +0.5% | +0.3% |
| lod_coarse_light/surface_level_3 | 99.1 us | 99.6 us | 99.4 us | +0.3% | +0.2% |
| lod_coarse_light/surface_level_4 | 103.7 us | 104.3 us | 103.6 us | -0.1% | +0.3% |
| lod_coarse_light/surface_level_5 | 103.9 us | 104.3 us | 104.7 us | +0.8% | +2.9% |
| lod_downsample/stair_surface | 331.6 us | 332.7 us | 331.4 us | -0.1% | -0.1% |
| lod_downsample/stairs | 606.4 us | 606.8 us | 603.2 us | -0.5% | +0.2% |
| lod_downsample/testgen | 291.0 us | 291.1 us | 288.8 us | -0.7% | -0.5% |
| lod_downsample/uniform_air | 3.1 ns | 3.1 ns | 3.1 ns | +0.2% | +0.9% |
| lod_downsample/uniform_halves | 32.6 us | 32.7 us | 32.1 us | -1.5% | -1.3% |
| lod_downsample/uniform_stone | 3.3 ns | 3.3 ns | 3.3 ns | +0.9% | +3.5% |
| lod_downsample/worldgen | 256.5 us | 257.3 us | 255.4 us | -0.4% | -0.4% |
| lod_mesh_coarse/surface_level_1 | 407.4 us | 411.5 us | 397.5 us | -2.4% | -3.2% |
| lod_mesh_coarse/surface_level_2 | 362.7 us | 365.1 us | 357.7 us | -1.4% | -1.7% |
| lod_mesh_coarse/surface_level_3 | 349.3 us | 350.7 us | 345.7 us | -1.0% | -1.2% |
| lod_mesh_coarse/surface_level_4 | 328.2 us | 329.5 us | 326.1 us | -0.7% | -0.8% |
| lod_mesh_coarse/surface_level_5 | 307.7 us | 307.9 us | 309.1 us | +0.4% | +0.8% |
| lod_mesh_coarse_lit/mouth_level_1 | 510.2 us | 511.0 us | 492.5 us | -3.5% | -3.1% |
| lod_mesh_coarse_lit/surface_level_1 | 475.0 us | 475.5 us | 460.6 us | -3.0% | -3.1% |
| lod_mesh_coarse_lit/surface_level_2 | 451.9 us | 452.7 us | 445.8 us | -1.4% | -1.2% |
| lod_mesh_coarse_lit/surface_level_3 | 450.5 us | 451.2 us | 445.5 us | -1.1% | -0.3% |
| lod_mesh_coarse_lit/surface_level_4 | 433.8 us | 434.2 us | 430.2 us | -0.8% | -0.5% |
| lod_mesh_coarse_lit/surface_level_5 | 412.1 us | 412.8 us | 417.3 us | +1.3% | +2.0% |
| mesh_chunk/cave_crossed | 391.7 us | 393.3 us | 391.3 us | -0.1% | +1.1% |
| mesh_chunk/cave_mouth | 369.2 us | 370.0 us | 367.1 us | -0.6% | -0.4% |
| mesh_chunk/declined_air | 12.2 ns | 12.4 ns | 12.1 ns | -1.2% | +0.7% |
| mesh_chunk/declined_enclosed | 12.4 ns | 12.5 ns | 12.4 ns | +0.1% | -0.2% |
| mesh_chunk/ramp_densest | 456.3 us | 457.1 us | 454.5 us | -0.4% | +0.5% |
| mesh_chunk/shore | 418.2 us | 420.4 us | 412.0 us | -1.5% | -1.2% |
| mesh_chunk/straddling | 411.6 us | 412.9 us | 409.9 us | -0.4% | -0.3% |
| mesh_chunk/straddling_unsmoothed | 360.7 us | 362.0 us | 360.2 us | -0.1% | -0.2% |
| neighborhood_isolated | 25.5 us | 25.8 us | 25.2 us | -1.2% | -1.6% |
| shape_register_fence | 611.5 us | 612.6 us | 595.1 us | -2.7% | -0.4% |
| shape_register_pipe | 2.40 ms | 2.42 ms | 2.36 ms | -1.6% | -0.9% |
| shape_table_builtin | 4.26 ms | 4.27 ms | 4.16 ms | -2.5% | -0.2% |
| snapshot/arc_clone_seven_chunks_and_lights | 35.2 ns | 35.8 ns | 34.6 ns | -1.5% | +5.7% |
| snapshot/clone_seven_chunks_and_lights | 4.5 us | 4.6 us | 4.5 us | -0.0% | +1.8% |
| snapshot/make_mut_shared | 306.4 ns | 322.0 ns | 300.7 ns | -1.8% | -4.5% |
| snapshot/make_mut_shared_light | 490.8 ns | 499.5 ns | 467.8 ns | -4.7% | -1.0% |
| snapshot/make_mut_unshared | 3.1 ns | 3.1 ns | 3.1 ns | -1.0% | -1.1% |
| testgen_terrain_chunk | 113.7 us | 114.0 us | 114.3 us | +0.6% | +1.1% |
| worldgen/cave_band_layer_of_64 | 8.87 ms | 8.91 ms | 8.80 ms | -0.7% | +0.6% |
| worldgen/cave_band_uniform | 29.9 ns | 30.0 ns | 29.9 ns | +0.1% | +1.1% |
| worldgen/cave_crossed | 264.5 us | 264.9 us | 265.2 us | +0.3% | +1.4% |
| worldgen/cave_mouth | 215.0 us | 216.3 us | 214.2 us | -0.4% | -0.3% |
| worldgen/early_out_air | 3.8 ns | 3.9 ns | 3.9 ns | +1.0% | +1.7% |
| worldgen/early_out_stone | 4.5 ns | 4.5 ns | 4.5 ns | +0.0% | +0.4% |
| worldgen/ramp_densest | 175.7 us | 176.0 us | 176.5 us | +0.5% | +1.6% |
| worldgen/shore | 135.1 us | 137.1 us | 134.2 us | -0.7% | -0.3% |
| worldgen/straddling | 169.1 us | 169.3 us | 166.5 us | -1.5% | -0.2% |
| worldgen/straddling_caved | 253.4 us | 254.4 us | 253.0 us | -0.1% | +0.4% |
| worldgen/straddling_unsmoothed | 105.3 us | 105.8 us | 105.1 us | -0.2% | +0.2% |

### Review notes

Two-axis review (standards, spec) on the finished tree; what was taken and
what was not:

- Taken: a stale `Palette::sum` comment, ticket and spec references in doc
  comments, "cell" for a Voxel position, the `jobs.rs` import order, `read`
  renamed `snapshot`, a duplicated air normalisation, and CONTEXT.md's
  Snapshot entry, which had become untrue (it holds the overlay now).
- Behaviour change to know about: a fluid Material placed with a Shape
  (`Voxel::new(WATER, STAIR, ..)`) used to be stored as a water stair; it is
  now a full source, as spec decision 8 asks of a 0.1 placement. No in-tree
  caller placed shaped water.
- `place_solid` displaces fluid for every solid today, including a stair, and
  a test pins that. Spec decision 5's "a partial Shape keeps compatible fluid"
  needs the passage geometry of ticket 04, which changes that one case.
- `FluidDef` carries the metre-valued reach, rates and reset drop of spec
  decision 4 now rather than a bare "is a fluid" flag: ticket 02 reads them
  on its first line, and the table is where the identity question had to be
  answered anyway. `Fluid` carries no propagation state yet; `Palette<T>`
  makes adding fields free of storage work.
- `Lifetime` has no consumer in this ticket beyond `unchanged_since`, where
  `Arc` identity already suffices. It stays because a simulation proposal
  (ticket 02 onwards) names Voxels and holds no Arc, and ticket 03 needs "a
  new Chunk at an old coordinate rejects old-lifetime Jobs".
- Declined: folding `RejectReason::message` into `PlaceFault`'s `Display`
  (the three parallel lists are the existing convention, not this ticket's),
  and making `ChunkStore::set_voxel`/`set_overlay` refuse a non-canonical
  pair (they are Server-private primitives; `set_occupants` is the one path
  and `Occupants::of` debug-asserts the invariant).
- For ticket 10: `Overlay`, `Occupants`, `Fluid`, source, `Burning` and
  `Lifetime` are new nouns CONTEXT.md does not have yet.
