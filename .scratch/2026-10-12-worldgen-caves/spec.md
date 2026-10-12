# Worldgen caves

Off the build order: DESIGN.md's ten steps do not mention caves. Done because
step 4 left light unobservable without a `dev` structure, so the branch takes no
`phase-N-` prefix: `feat/worldgen-caves`.

Started after step 5 (shaped voxels, `047725f`) and after off-thread streaming
(`d8b3f31`), in that order, on purpose. Step 5 changed `blocks_light`, so a
light bug on cave terrain has one suspect and not two. Off-thread streaming took
generate, relight and mesh off the frame-critical path, which turned the
question this task exists for from "does the tick still fit" into "how much
worker time does a Chunk cost, and how stale can a priority get".

## Why

1. **Light propagation is unobservable in the real world.** `worldgen` is a
   heightmap, so every visible face has an open column above it and sky light is
   15 everywhere. Step 4 ships `server/lightbox.rs`, a `dev`-only shelter,
   purely so a screenshot can show the feature. Caves make the flood-fill carry
   itself.
2. **Dirt and Stone never appear on screen.** Grass covers soil covers stone.
   Two of the three placeholder Material colours are only unit-tested.

## The problem

Three fast paths key off one property - deep rock is a uniform Chunk - and
carving into it breaks all three:

| Fast path                      | Uniform today | Not uniform |
| ------------------------------ | ------------- | ----------- |
| `Worldgen::chunk` deep early-out | 3.8 ns      | 100-200 us  |
| `light_chunk` opaque path      | 3.1 ns        | 50-140 us   |
| `mesh_chunk` enclosed case     | 5.9 ns        | 350-410 us  |

Most of a 3D ring is below the surface. Before off-thread streaming the sum
sat on the tick: `24 x generate + 12 x relight + 12 x mesh` was 8.1 ms against
15.6 ms. After it, the same sum is worker time, and the cost of getting it wrong
is fill latency and a longer staleness window (`cap x mean Job cost / threads`,
`StreamingBudget`'s doc comment), not a dropped frame. The early-out is still
the design work: it is the difference between a cave costing a fifth of a ring's
Chunks 0.6 ms each and costing all of them.

## Baseline

`cargo bench -p tesserae-core` on `main` (`d8b3f31`), Apple M5, release,
2026-10-12. Unchanged benches are not repeated in the PR.

| Bench                             | Time     |
| --------------------------------- | -------- |
| `greedy_mesh/terrain`             | 335.4 us |
| `greedy_mesh/solid`               | 319.9 us |
| `greedy_mesh/checkerboard`        | 2.76 ms  |
| `greedy_mesh/terrain_light_grad`  | 352.3 us |
| `greedy_mesh/terrain_stairs_cap`  | 518.4 us |
| `mesh_chunk/straddling`           | 349.3 us |
| `mesh_chunk/declined_air`         | 6.9 ns   |
| `mesh_chunk/declined_enclosed`    | 5.9 ns   |
| `neighborhood_isolated`           | 26.9 us  |
| `worldgen/straddling`             | 103.1 us |
| `worldgen/early_out_air`          | 3.8 ns   |
| `worldgen/early_out_stone`        | 3.8 ns   |
| `testgen_terrain_chunk`           | 122.2 us |
| `light_chunk/straddling`          | 89.3 us  |
| `light_chunk/straddling_settled`  | 140.6 us |
| `light_chunk/straddling_shaped`   | 148.6 us |
| `light_chunk/open_sky`            | 3.9 ns   |
| `light_chunk/opaque`              | 3.1 ns   |
| `light_chunk/flooded_air`         | 118.3 us |

(`mesh_chunk/straddling` read 682 us in the first full run and 349 us when run
alone in a clean checkout of `main`; the first was disturbed. Every number to
compare against is taken in isolation.)

## Prototype findings

Measured with a throwaway example (not committed) over a 12 x 12 column, nine
Chunk-high volume at 1 m Voxels, seed `0x7e55e2ae`, to pick the design before
writing it. They are why the decisions below are what they are.

- **A bound from lattice corners is useless; an exact one is not.** The first
  bound for "can this box contain a cave" took the range of the noise lattice
  values in every cell the box touches. Value noise lattice values are uniform
  in [0, 1), so a cell's range nearly always contains the tunnel's midline, and
  the "early-out" cost 30 us on average and ruled out almost nothing. Inside one
  lattice cell value noise is multilinear in three smoothstepped weights, and a
  multilinear function on a box takes its extremes at the corners of the box. So
  the exact range over a box is eight evaluations of the cell's own corners at
  the box's weights. Averaged over every uniform-stone Chunk of the sweep
  (mostly trivially uniform, below the floor) that took 30 us to 1.4 us, and the
  Chunks produced were identical, because the bound only decides cost. The
  per-layer figures below are the honest ones.
- **The depth floor does most of the early-out's work; the range test is the
  second line, and a thinner one than first measured.** A first run counted 105
  of 576 "deep" Chunks as carved (18%), but three of the four layers counted
  were below the floor and trivially uniform. The honest figure is per layer
  inside the band: at the default (64 m features, floor -24) the one deep layer
  that is in the band has 186 of 256 Chunks carved (73%) and 70 uniform; at 96 m
  features 40% are carved; at 128 m, 24%. So tunnels this size are close to
  everywhere in the band, the range test keeps 27% to 76% of it uniform
  depending on spacing, and **a deep floor is the lever that keeps a ring
  uniform**. A deep floor of -120 puts four layers in the band instead of one.
- **What a Chunk costs by outcome**, release, 1 m Voxels: a carved one takes
  200-330 us to generate; the in-band uniform ones average 6-35 us, a mix of
  ruled out by the range test and passed it but carved nothing; over a layer the
  sub-block culling is 390 us a Chunk against 640 us for sampling every Voxel,
  a modest 1.65x. Carved Chunks cost about 50 us to light against dark
  neighbours and 408 us to mesh. A second culling level over octants was tried
  and moved nothing (under 10%): what a false positive costs is the Voxels
  sampled in sub-blocks the bound is too loose to exclude, not the bound.
- **Tunnels, not blobs.** Two independent fields that are both near their
  midline cross in a winding tube; one field above a threshold is a blob. A
  tube is thin, so little of the volume is carved even when most Chunks hold
  some: at `feature_size_m = 64`, `radius_m = 3` about 4.6% of the ground in the
  band is carved and about 4% of surface columns have a mouth.

## Decisions

### 1. What a cave is, in metres

A tunnel: the points where two independent 3D fields (`fbm3`, two octaves) are
both within a window of 0.5. Three physical parameters on `CaveParams`, all
metres, none ever a Voxel count, per ADR-0003:

- `feature_size_m` (64): the period of the field's lowest octave, which is
  roughly how far apart two tunnels run.
- `radius_m` (3): roughly how wide a tunnel is. **Approximate by construction:**
  the window is `GRADIENT * radius_m / feature_size_m` and `GRADIENT` is a
  calibration of how steep value noise is, not a derivation. The size is the
  same physical size at every Voxel size, which is the property ADR-0003 needs;
  it is not a measured radius, and the docs say so.
- `floor_m` (-24, i.e. `sea_level_m` less 48): no cave below this height.

`octaves` (2) is a fourth field, a count and not a distance, like the terrain's.
Two keep the exact bound (decision 2) cheap: its cost is one piece per lattice
cell per octave, and a Chunk spans about one cell of the first and two of the
second.

The field is sampled at a Voxel's lower corner, as the surface is, so the same
metre point gets the same answer at any Voxel size. A test pins it, beside the
existing "surface lands at the same height at any Voxel size".

### 2. How the deep early-out survives

Three layers, cheapest first. The result is **identical** to sampling every
Voxel; only the cost differs, and a test builds the slow reference and compares.

1. `floor_m`: a Chunk wholly below it, or above the surface band, answers as
   today (3.8 ns). Not a heuristic: the cave does not exist there.
2. A **range bound** over the Chunk's box (`noise::fbm3_range`): exact per
   octave, summed over octaves, so sound (it never says "cannot" for a box that
   can) and slightly loose (it sometimes says "can" for one that does not). If
   either field's range stays clear of the window, the Chunk is `Chunk::filled`
   stone with no sampling. About a microsecond.
3. A Chunk that passes is generated as **filled stone, then carved**, visiting
   only the 8 x 8 x 8 sub-blocks the same bound cannot rule out. Starting from a
   filled Chunk and not an air one matters: a Chunk that passed the bound but
   carves nothing is still `uniform()`, so the light and mesh fast paths still
   fire for it.

This is the "coarse per-Chunk test" the brief listed, made exact instead of
sampled, plus a depth floor. The third option (give up the early-out and pay) is
paid for one layer of Chunks and not for the ring: at the default floor the
Chunks a ring holds below the soil line are one layer in the band and the rest
under it, and a layer in the band costs about 0.3-0.6 ms per carved Chunk across
the three stages, a once-per-load cost on workers.

### 3. Caves breach the surface

A cave carves any Voxel below the surface, including the Grass cap. A mouth is
what makes caves visible from outside and what puts a sky-light gradient on
screen. The cost is the invariant `exactly_one_grass_voxel_caps_every_column`:
it becomes "**at most** one, and it is the highest solid Voxel", with a second
test that with caves off it is exactly one. A column under a mouth has no Grass,
and the soil below it shows as Dirt.

Nothing reaches above `surface_bounds_m().1`, structurally: a cave can only
turn a Voxel the surface logic made solid into air, so the generator still makes
nothing but air above the band, and `streaming::relight`'s `above_the_surface`
stays exact. A test asserts it on a sweep of Chunks anyway.

### 4. No lining

Air is the carve and the walls are whatever was there: Dirt within
`soil_depth_m` of the surface, Stone below. That is what fills in the second
reason above without a new Material. A lining is a Mod's decision once Materials
are declared (step 7).

### 5. Caves are a `WorldgenParams` field

`caves: Option<CaveParams>`. Plain `Copy` data, no callbacks, filled in once by
a Mod; `None` is a World without caves and costs the generator one `Option`
check. The default has caves on, which changes every default World, and
`generation_places_no_shaped_voxels`, `generation_is_deterministic` and the
streaming tests ride along unchanged.

### 6. The budget

Re-sized against measurement in ticket 05, and stated honestly there: the
per-Chunk cost went up for the minority of Chunks a tunnel crosses, so the mean
Job cost the staleness window multiplies by went up a little, and the mesh
landing cap's 6-12 us upload is re-measured because a carved Chunk has more
faces.

### 7. `testgen` does not change

`terrain_output_is_pinned` guards the mesher benches' fixture. Caves belong in
`worldgen`, tuned in metres; `testgen` is tuned in Voxel indices on purpose.

### 8. `noise.rs` gains a 3D primitive, with no dependency

`noise3`, `fbm3`, `fbm3_range`. Core stays at zero dependencies. `fbm3_range`
is the one new idea in the file and its soundness is property-tested: sample
many boxes, many points in each, assert every value lies in the box's range.

## Invariants, and where each is enforced

1. **Core acquires no `bevy` or `wgpu`, and has no dependencies.** CI's
   `Boundaries` job.
2. **The Server is the sole authority over World state.** Nothing here touches
   `ChunkStore`; generation was already Server-side.
3. **Everything physical is in metres.** `CaveParams` is metres; the 8-Voxel
   sub-block is an implementation detail that cannot change an output, and the
   Voxel-size test proves it.
4. **No Mod reaches core directly.** `CaveParams` is data.
5. **Light's two solvers agree.** Ticket 04 extends the agreement test to
   generated cave terrain.

## Performance

Bench ticket 03 adds a Chunk a cave crosses to `worldgen`, `light_chunk` and
`mesh_chunk`, plus the cost of the bound itself and of a Chunk that passes it
and carves nothing. Before/after table in the PR.

## Done when

`cargo dev` shows caves with sky light falling off into them and Dirt and Stone
in their walls; `./scripts/cargo-ci` passes; `cargo bench -p tesserae-core`
states what the streaming budget became; a screenshot is taken and shown; and
`server/lightbox.rs` has been decided on deliberately, either way (ticket 06).
