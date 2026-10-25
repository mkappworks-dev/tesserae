# 11: Give dry, unburning Chunks back their old mesh and light cost

Status: done
Blocked by: 09

Spec: [decision 3 and measurement policy](../spec.md).

## What to build

Ticket 09 measured that the phase made meshing and the bulk light solve
10-30% slower on Chunks that hold no fluid or fire, and Shape registration
40-90% slower. The cost came from tickets 02 to 07 (light reads the overlay
and wet light parameters per Voxel, `Neighborhood` scans seven overlays and
keeps fluid side arrays, registration flood-fills every variant's voids).
Spec decision 3 says dry terrain must keep cheap storage and idle
behaviour; this ticket gives it back its cheap hot paths too.

Added 2026-10-24 on the user's instruction, after ticket 09, to land before
ticket 10 so the documented budgets do not go stale.

## Acceptance criteria

- [x] A Chunk whose overlay and whose neighbours' overlays are uniform
  `Overlay::NONE` takes mesh and light paths whose cost matches the
  pre-phase baseline (2ae7399) within measurement noise, with no change
  to their output.
- [x] Wet or burning Chunks, and dry Chunks beside wet ones, produce exactly
  the output they produce today; tests pin fast-path and general-path
  equality on dry, mixed and boundary fixtures.
- [x] Shape registration cost is reduced where a cheap, safe change exists
  (e.g. deriving voids lazily or sharing them across equivalent variants);
  otherwise the remaining startup cost is measured and recorded.
- [x] Interleaved thread-CPU A/B against 2ae7399 and b454e28, uptime and load
  beside every figure, raw records kept, distorted pairs identified.
- [x] No Server-tick budget, bound or simulation behaviour changes.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- 2026-10-24: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- Profiling first: a per-ticket bisect (thread-CPU criterion, every phase
  commit built from `git archive`) put the cost on three tickets, and on
  code generation more than on overlay work. The dry gates the tickets
  added (`may_contain_fluid` / `may_contain_burning` from the overlay's
  palette, centre and neighbours) already kept dry Chunks off the wet
  paths; what changed was how LLVM compiled the paths around them.
  - `neighborhood_isolated` +60% from ticket 02: the centre copy read a
    Voxel at a time through `Chunk::get`, and once the overlay copy sat
    beside it the optimiser stopped hoisting the palette's match out of
    the loop.
  - Dry light +20-60% from ticket 07: the sweep matched on `wet` per Voxel
    for the fluid and burning reads, and the loop lost its unswitching.
  - Registration +40-70% from ticket 04: `find_voids` flood-filled every
    variant one sub-cell at a time.
  - `greedy_mesh` +10-15% with an unchanged inner loop: pass 1 kept the
    cell's coordinates in an array indexed by axis, which lived on the
    stack, so its speed followed how the function around it was laid out.
- `Palette::read_run` and `Chunk::read_row` (new): a row of Voxels in one
  match on the storage; the Indexed branch shifts instead of dividing,
  since `bits` divides 64 (`read_index`'s division per value cost the
  light sweep 10-20%). `ChunkLight::read_row` copies a row of light.
- `Neighborhood::new` copies the centre Chunk and its light by rows.
- `sweep<TRANSLUCENT, WET>`: `WET` is a const generic, chosen once in
  `solve` from whether `wet` is `Some`, so a dry Chunk's sweep is the
  per-Voxel loop it was before overlays and never reads one; it also reads
  by rows. `solve` itself stays one function: making it generic on `WET`
  pushed `flood_as` out of line and measured 1.5-3x slower.
- Pass 1 of `greedy_mesh` indexes the padded arrays by `STRIDES`, not by a
  coordinate array; the array is still built for the rare slow-path calls.
  `#[inline(never)]` on pass 1 was tried and measured slower.
- `find_voids` floods a row of sub-cells at a time as a bitmask
  (`runs_from`, the carry trick in both directions). It is
  `#[inline(never)]`: inlined into `ShapeTable::derive` it slowed the
  other loops there 10-20%. Voids are still derived eagerly for every
  variant; deriving them lazily would move the cost into the first
  mesh or light that asks, on a worker, and was not needed once the flood
  was cheap.
- Deviation: no duplicated dry copy of any path. The gates are the
  existing palette checks; the const generic is the sweep's only new
  variant. Every other change is the same code for wet and dry Chunks.

### Verification

- Test functions: core lib 320 (317), meshing 68 (66), Mods 89 (three
  ignored), `tesserae` binary 347 (two ignored); workspace 824 (819). Full
  `scripts/cargo-ci` passed on the tree this commit records.
- New tests:
  - `light::tests::a_dry_chunk_lights_the_same_by_the_dry_sweep_and_the_
    wet_one`: generated Chunks with caves, stairs and slabs, and a
    hand-built room of glass, leaves, a lantern, stairs, a slab, a glass
    stair, a diagonal and a post, each with and without a lit border:
    `light_chunk` == `solve` with an all-dry overlay == `light_cells` with
    a drained overlay (palette still listing water and fire, so it takes
    the wet sweep everywhere).
  - `meshing::a_dry_neighbourhood_meshes_the_same_whichever_path_it_takes`:
    generated ramps and two more Chunks, and a hand-built Chunk of fences,
    glass, leaves, water and a glass stair beside stone, sky-lit and
    meshed with every overlay dry, the centre drained, each neighbour
    drained in turn, and all drained; the meshes are equal.
  - `meshing::a_dry_chunk_beside_a_wet_one_meshes_against_the_neighbours_
    fluid`: packed water on the +x face, one face against a dry neighbour,
    none against flowing water there.
  - `chunk::tests::a_row_read_matches_reading_each_voxel`: 1, 2, 3, 5, 17
    and 300 kinds (every palette width), with guard cells around the row.
  - `shape::tests::the_row_flood_finds_the_voids_a_cell_flood_does`
    against the old per-cell flood kept as a test reference: slab, stair
    and post in every orientation, fence and pipe under every arm mask,
    wall partitions, a hollow box, empty and full grids, 500 random box
    sets.
- Mutations, each caught: the light gate ignoring burning (2 binary tests
  fail); the wet sweep reading no overlay (3); the mesh gate reading only
  the centre's overlay (the boundary test); the row flood dropping its
  seeds (the new flood test and others); a row read off by one (93); the
  row read's shift taken modulo 32 (15 core tests).
- Output identity: a digest harness hashed mesh, light and void output for
  352 cases (dry, wet, burning, mixed and boundary fixtures, the builtin
  Shape table) on b454e28 and on this tree; all 352 are identical.
- Nothing under `server/`, no budget, bound, tuning or simulation code is
  touched; Server ticks are unchanged by construction and the binary's
  tests pass unchanged.

### Measurement method

- Thread-CPU criterion (ticket 01's uncommitted patch, applied to each
  side's `git archive` copy and never committed), release builds, one
  process per side and bench group, interleaved in a rotating order over
  four rounds, `uptime` before and after each process. A process waits up
  to 10 minutes for the 1-minute load to fall under 6 (the wait is logged).
  The tables give the minimum over rounds of criterion's per-round median,
  and the median of those.
- Sides: 2ae7399 (before the phase), b454e28 (ticket 09), this tree.
- Raw records: `/private/tmp/tesserae-p8-t11.a/` (bisect, quick
  experiments), `/private/tmp/tesserae-p8-t11.d/` (`ab.log`, `ab-*.out`:
  first full A/B, all groups; `ab2.log`, `ab2-*.out`: final meshing and
  LOD A/B on the final tree), `/private/tmp/tesserae-p8-t11.c/` (digests),
  `/private/tmp/tesserae-p8-t11.b/` (mutations).
- Distorted or discarded runs:
  - First A/B, round 1 this-tree meshing straddled a clamshell sleep
    (18:44-20:02): discarded.
  - First A/B, round 3 was killed at the 2-hour job limit: its partial
    outputs were discarded and rounds 3-4 rerun.
  - Final A/B: round 1 b454e28 LOD (load 4.9 to 119), round 3 2ae7399 LOD
    (4.9 to 175) and round 3 b454e28 meshing (5.7 to 31.8) ended under
    load from other work; kept, and the minimum over rounds is what the
    tables compare.
  - Quick experiments at load 14-20 (`#[inline(never)]` on pass 1) are
    not used for any conclusion beyond being rejected.
  - The first A/B's light rows were measured before the shift-based
    `read_run` and are superseded by the final A/B.

### Evidence

Final A/B, meshing group (load 4.3-12.8 at the start and end of 2ae7399
runs, 4.4-31.8 b454e28, 4.3-18.6 this tree). Min per side; Δ vs 2ae7399 as
min/median; Δ vs b454e28 as min.

| bench | 2ae7399 | b454e28 | this | vs 2ae7399 | vs b454e28 |
| --- | ---: | ---: | ---: | ---: | ---: |
| greedy_mesh/all_stairs | 3.81 ms | 3.69 ms | 3.61 ms | -5/-6% | -2% |
| greedy_mesh/checkerboard | 2.72 ms | 2.71 ms | 2.75 ms | +1/-6% | +1% |
| greedy_mesh/flat_fences_at_cap | 593 us | 587 us | 461 us | -22/-19% | -21% |
| greedy_mesh/glass_room | 290 us | 318 us | 161 us | -44/-42% | -49% |
| greedy_mesh/solid | 351 us | 380 us | 224 us | -36/-38% | -41% |
| greedy_mesh/terrain | 370 us | 389 us | 221 us | -40/-40% | -43% |
| greedy_mesh/terrain_canopy | 455 us | 496 us | 329 us | -28/-28% | -34% |
| greedy_mesh/terrain_fences_at_cap | 675 us | 658 us | 483 us | -28/-30% | -27% |
| greedy_mesh/terrain_light_gradient | 379 us | 394 us | 243 us | -36/-46% | -38% |
| greedy_mesh/terrain_stairs_at_budget | 500 us | 544 us | 374 us | -25/-28% | -31% |
| greedy_mesh/terrain_stairs_at_cap | 613 us | 680 us | 523 us | -15/-27% | -23% |
| greedy_mesh/terrain_water | 456 us | 458 us | 295 us | -35/-40% | -36% |
| light_chunk/cave_dark_neighbours | 49.2 us | 69.1 us | 43.1 us | -12/-13% | -38% |
| light_chunk/cave_mouth | 119 us | 142 us | 111 us | -7/-6% | -22% |
| light_chunk/flooded_air | 138 us | 130 us | 134 us | -3/-9% | +2% |
| light_chunk/leaves_canopy | 184 us | 211 us | 202 us | +10/+3% | -4% |
| light_chunk/opaque | 3.6 ns | 3.9 ns | 4.2 ns | +18/+17% | +9% |
| light_chunk/open_sky | 4.9 ns | 4.2 ns | 4.4 ns | -10/-7% | +5% |
| light_chunk/ramp_densest | 219 us | 245 us | 207 us | -6/-7% | -15% |
| light_chunk/straddling | 203 us | 235 us | 203 us | -0/-1% | -14% |
| light_chunk/straddling_settled | 329 us | 348 us | 324 us | -2/-2% | -7% |
| light_chunk/straddling_shaped | 174 us | 198 us | 163 us | -6/-3% | -18% |
| light_chunk/straddling_unsmoothed | 94.0 us | 122 us | 98.8 us | +5/+0% | -19% |
| light_chunk/straddling_water | 144 us | 168 us | 146 us | +1/+6% | -13% |
| mesh_chunk/cave_crossed | 417 us | 469 us | 266 us | -36/-34% | -43% |
| mesh_chunk/cave_mouth | 386 us | 439 us | 251 us | -35/-41% | -43% |
| mesh_chunk/declined_air | 13.3 ns | 23.2 ns | 22.8 ns | +71/+80% | -2% |
| mesh_chunk/declined_enclosed | 12.9 ns | 23.1 ns | 23.6 ns | +84/+71% | +2% |
| mesh_chunk/ramp_densest | 468 us | 544 us | 333 us | -29/-31% | -39% |
| mesh_chunk/shore | 440 us | 516 us | 308 us | -30/-31% | -40% |
| mesh_chunk/straddling | 445 us | 476 us | 287 us | -35/-32% | -40% |
| mesh_chunk/straddling_unsmoothed | 368 us | 427 us | 236 us | -36/-42% | -45% |
| neighborhood_isolated | 26.7 us | 41.1 us | 19.8 us | -26/-27% | -52% |
| shape_register_fence | 625 us | 1.04 ms | 722 us | +15/+13% | -31% |
| shape_register_pipe | 2.46 ms | 4.18 ms | 2.71 ms | +10/+10% | -35% |
| shape_table_builtin | 4.44 ms | 5.97 ms | 4.81 ms | +8/+13% | -19% |

- Snapshot, worldgen and `testgen_terrain_chunk` rows (not touched) are
  within -12% to +8% of both sides on min and median.
- Rows over 2ae7399, and why:
  - `light_chunk/leaves_canopy` (+10% min, +3% median) and
    `straddling_unsmoothed` (+5% min, +0% median): within the rounds'
    spread. 2ae7399 measured 184-227 us and 94-104 us over its four
    rounds, this tree 202-208 us and 99-107 us.
  - `light_chunk/opaque` and `mesh_chunk/declined_*` are 4-23 ns early
    returns, not the mesh or light paths, and move with where the
    binary places them. `declined_air` measured 13.3 ns on b454e28 in the
    bisect build and 23.2 ns on the same commit here; this tree measured
    13.4-14.1 ns in a quick run and 22.8 ns here. 2ae7399 measured 12.4-13.3
    ns in every run. The `may_contain_fluid` check ticket 02 put in front
    of `emits_no_faces` is the only code difference on that path, a
    palette read.
  - Registration stays 8-15% over 2ae7399 (from 34-70%): that is the void
    derivation itself, a cost spec decision 3's geometry needs, paid once
    at startup (4.8 ms for the builtin table).

Final A/B, LOD group (load 4.4-175 at 2ae7399's run edges, 4.2-119
b454e28, 5.6-13.8 this tree; the two high figures are the distorted runs
above).

| bench | 2ae7399 | b454e28 | this | vs 2ae7399 | vs b454e28 |
| --- | ---: | ---: | ---: | ---: | ---: |
| mesh_coarse/surface_level_1 | 418 us | 464 us | 284 us | -32/-34% | -39% |
| mesh_coarse/surface_level_3 | 364 us | 421 us | 226 us | -38/-36% | -46% |
| mesh_coarse/surface_level_5 | 344 us | 384 us | 190 us | -45/-43% | -50% |
| mesh_coarse_lit/mouth_level_1 | 600 us | 553 us | 383 us | -36/-37% | -31% |
| mesh_coarse_lit/surface_level_1 | 548 us | 539 us | 359 us | -35/-41% | -33% |
| mesh_coarse_lit/surface_level_5 | 474 us | 480 us | 292 us | -38/-39% | -39% |
| downsample/worldgen | 273 us | 211 us | 207 us | -24/-24% | -2% |
| coarse_light/surface_level_1 | 65.2 us | 64.3 us | 62.7 us | -4/-2% | -2% |

- Every `mesh_coarse` and `mesh_coarse_lit` level is 32-45% under 2ae7399;
  `coarse_chunk`, `coarse_light` and `downsample` rows are within -12% to
  +3% of b454e28 and at or under 2ae7399.

First A/B, unchanged paths (load 1.5-6.0 at every run edge, 10.5 once at a
b454e28 fluids end): `collide/*` -0.9% to -5.7% min vs 2ae7399;
`fluids/*` and `fire/*` -2.5% to +4.3% min vs b454e28 (medians -1% to
+8.9%). Neither group calls the row reads.
