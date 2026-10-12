# Worldgen slope smoothing (stairs and slabs where the ground steps)

Off the build order, on the branch `feat/worldgen-slope-smoothing`. Step 5
(shaped voxels, `047725f`) left terrain with no shaped Voxel on purpose: the cap
(`MAX_SHAPED_PER_CHUNK`) and the second mesher pass were built and measured
against hand-made scenes, and `generation_places_no_shaped_voxels` pinned worldgen
to cubes so every bench input from it was unchanged. Caves (`f232c4e`) then made
light observable. This does the same for shapes: it is the first time pass 2 of
the mesher, per-face light sealing and per-Shape occlusion meet real ground.

## Why

1. **Shaped Voxels are only ever seen in the `dev` gallery or after a player
   places one.** The mesher's second pass, the seal-aware light solvers and the
   coverage rule for culling have never run on terrain, which is the input
   everything downstream is tuned against.
2. **A cube heightmap is a staircase of one-Voxel cliffs.** A 1 m step is
   awkward to look at and, once there is a player, to walk up. A stair or slab
   halves it. This is the use DESIGN.md names ("slope-smoothing stairs and
   slabs are the obvious worldgen use").

## The rule

Worldgen already computes, per column, a surface height in metres and fills
every Voxel whose lower corner is at or below it. The topmost solid Voxel of a
column is `t`. Compare a column with its four **cardinal** neighbours (the
diagonals are not asked: a stair has one tall side, and a diagonal has no side).

A neighbour **rises one Voxel** over this column when its topmost solid Voxel is
`t + 1`, i.e. its ground is exactly one cell higher. Then, in the cell above this
column's top (`t + 1`), the Voxel abuts that neighbour's top cube at the same
level, and a half-height Shape there turns a one-Voxel cliff into two half steps:

- **Exactly one neighbour rises:** a **stair**, its tall side towards that
  neighbour. A walker climbs half a Voxel onto the tread, then half a Voxel onto
  the neighbour.
- **Two or more rise** (a channel, a corner, a notch): a **slab**, lying on its
  lower half. A stair has one tall side and no corner variant exists, so any
  stair here would leave one rising side a full cliff; a slab is the same half
  step on every side and is correct for all of them.

Both are the Material of the column's top cube (Grass), so the ramp reads as
ground. A neighbour that rises two or more Voxels is a cliff and is not eased:
a half step does not make it walkable and a stair there would be decoration.

A stair or slab is placed only if the cube under it **and** each abutting cube
are really solid. Caves carve through the Grass cap, and a mouth next to a
ramp would otherwise leave a stair floating over a hole or leading into air.

## Decisions

### 1. The threshold is a rise in metres, and is the only knob

`WorldgenParams::smoothing: Option<SmoothingParams>`, plain `Copy` data like
`caves`, `None` for a World of cubes, on by default.

```rust
pub struct SmoothingParams {
    /// The smallest step in the ground, in metres, worth easing.
    pub min_rise_m: f32,
}
```

A step's rise is `(t_neighbour - t) as f32 * voxel_size`. A stair and a slab
are half a Voxel tall, so what a Shape can ease is a rise of exactly one Voxel
(that is geometry, not a tuned Voxel count), and the rise in metres is
`voxel_size`. The pass runs when that rise is at least `min_rise_m`. So the
threshold is a physical height: at the default 1 m Voxel every one-Voxel step
qualifies, and at 0.25 m Voxels a 0.25 m step does not, because a body steps
over that unaided and there is nothing for a stair to fix. Default **0.75 m**: above the 0.5 m a slab
already gives at the default Voxel size and below the 1 m of a cube, so a
1 m world is smoothed and a 0.5 m one is not. It is a placeholder for the step
height a player controller will own (step 5, decision 9, "the number is in
metres when it arrives"); that controller should set this from the same
constant.

No Voxel count appears in the rule: the neighbour test compares topmost indices
(a count of cells, but the same test that decides where the cubes are), and the
decision whether to apply it compares metres. A test sweeps `min_rise_m`
against Voxel sizes and asserts the pass fires exactly when the rise in metres
clears it.

### 2. The pass reads surface heights, not Voxels

The generator needs the surface height of the columns **just outside** the Chunk
(a stair on the edge column faces a neighbour that lives in the next Chunk). It
already has `surface_height_m` as a pure function of a metre position, so the
Chunk samples a one-column border: (CHUNK_SIZE + 2)^2 fbm2 evaluations instead
of CHUNK_SIZE^2, about 13% more noise, no dependency on a neighbouring Chunk and
no ordering. Two neighbouring Chunks decide a shared edge from the same two
numbers, so they agree, and a test checks it.

The topmost index `t` per column must agree with how the cube pass decides
solidity. The cube pass fills Voxel `y` when `surface_m >= floor_m + y * voxel_size`
(`below_m >= 0`); the pass derives `t` from that very comparison, and not from
`floor(surface / voxel_size)`, so the two cannot disagree by an ulp.

The pass runs after the cube pass, over the columns, on the Chunk that
contains cell `t + 1`. A column whose top is the last Voxel of the Chunk below
puts its stair at `y = 0` of the Chunk above, and the Chunk above computes that
by itself from the same surface array (the column's `t` is -1 in its frame).

### 3. The early-outs stay sound without moving

The sky early-out (`floor_m > highest`) and the deep one are unchanged. A stair
requires a neighbour cube in the same cell, so the cell's lower corner is at most
that neighbour's surface, which is below `highest`; a Chunk wholly above the
band therefore holds none. A deep stone Chunk is below `lowest - soil_depth` and
holds no surface. A test asserts both over a sweep, and
`surface_bounds_m`'s doc gains the sentence, because the Server's open-sky
boundary leans on "nothing solid above the band" and a stair is solid.

### 4. Orientation: the table decides, the generator does not guess

The stair's canonical tall side is +Z. Facing +X, -X, +Z or -Z is `up = +Y` at
a `turn`. The mapping from a direction to a `turn` lives in one function and a
test checks it against the **built ShapeTable**: for each of the four
directions, the placed stair seals the face towards the rising neighbour and
leaves the opposite face open, so a wrong turn is a failing test and not a
staircase that faces the wrong way. A slab is the identity orientation (lower
half).

### 5. The cap: structural, and a budget below it

`MAX_SHAPED_PER_CHUNK` stays **1024**. Placing at most one Voxel per column
bounds the pass at `CHUNK_SIZE^2` = 1024, which happens to equal the cap
exactly.

Equal to the cap is not good enough. At the cap a player can place nothing in the
Chunk (every Place is `TooManyShaped`), and a 45-degree ramp, which a Mod can
ask for with a large relief over a small feature size, makes every column a
candidate. So the pass also keeps its own budget, `MAX_SMOOTHED_PER_CHUNK =
MAX_SHAPED_PER_CHUNK / 2`: half the cap is the generator's, half is the
player's. Columns are visited in a fixed order (z-major), so when the budget
runs out the rest stay cubes and the output is still a pure function of
`(seed, coordinates)`. The cost side is the cap's own sizing rule: a Chunk at
the cap meshes within +50% of `mesh_chunk/straddling`, measured 41% at 1024, so
a worst case of 512 is about +20% by the step-5 numbers (linear, ~170 ns a
stair). This ticket measures the real figure, which is lower because real
terrain does not saturate.

The cap value is not changed. What the sweep proves is that real default terrain
sits far under the budget, so the budget is a safety valve and not a thing
players will see; the saturated case is tested directly on the pass with a
synthetic 45-degree surface.

`generation_places_no_shaped_voxels` is replaced, deliberately:

- `without_smoothing_generation_places_no_shaped_voxels`: the old contract for
  `smoothing: None`, so "None is a World of cubes" is pinned.
- `smoothed_generation_stays_within_the_budget`: a sweep over Chunks at three
  Voxel sizes asserting `shaped_count() <= MAX_SMOOTHED_PER_CHUNK <=
  MAX_SHAPED_PER_CHUNK`, that some Chunk holds shaped Voxels, and printing the
  densest Chunk so the PR can quote it.

### 6. Existing worldgen tests keep their meaning

The `terrain()` and `caved()` test helpers set `smoothing: None`: they test the
surface, the soil depth and the caves, and a stair on top of the Grass cap would
break "exactly one Grass Voxel" and "highest solid is Grass" for reasons that
are the point of this feature. New tests use `smoothed()` and `smoothed_caved()`.
The `cave_worlds()` helper in `light.rs` and the cave benches pin
`smoothing: None` for the same reason: they diff a caved World against a
cave-free one.

What does ride along on the default: `declining_never_changes_the_mesh_it_would_have_produced`
(tests/meshing.rs) and the light agreement tests that use
`WorldgenParams::default()` now run on terrain with shaped Voxels. That is the
point of the ticket 03, which adds explicit ones for the new input rather than
trusting the ride-along.

### 7. `testgen` does not change

`terrain_output_is_pinned` guards the mesher benches' fixture; `testgen` is
tuned in Voxel indices on purpose. Untouched. `greedy_mesh/terrain*` benches
read `testgen` and so do not move; `worldgen/*`, `mesh_chunk/*`,
`light_chunk/*` read `Worldgen` and do.

### 8. A dev switch for the A/B

`TESSERAE_SMOOTHING=0` (dev feature only), beside `TESSERAE_CAVES=0` in
`server::worldgen_params`, so the streaming budget is measured with and without
on one binary, as the caves branch did.

## Invariants, and where each is enforced

1. **No Client system mutates World state.** Nothing here touches the Client or
   `ChunkStore`; generation was already Server-side.
2. **No Mod reaches `tesserae-core` directly.** `SmoothingParams` is data,
   handed over once.
3. **`tesserae-core` depends on neither Bevy nor wgpu.** CI's `Boundaries` job.
4. **Everything physical is in metres.** `min_rise_m` is metres; the surface
   array's border and the budget are implementation details that cannot change
   what a given column gets (the budget changes only saturated Chunks, and is
   documented as such).
5. **Light's two solvers agree**, and **no exposed face is missing or drawn
   twice** on the new input. Ticket 03.

## Performance

The first `--save-baseline` run was taken while three sessions were
benchmarking and compiling on one machine (load average above 50) and was
discarded. The baseline is `main` (`f232c4e`) built in a separate worktree, and
the numbers that go in the PR come from running both bench binaries back to back
on a quiet machine, with the load average stated beside them. Rows that change
because their *input* is now smoothed terrain, rather than because code got
slower, are labelled as such; to separate the two, ticket 04 adds
`worldgen/straddling_unsmoothed` (the generator's own cost) next to the existing
`straddling` (now with the pass), and `mesh_chunk`/`light_chunk` rows for both
inputs. The streaming worst tick is re-measured (ticket 05) with
`scripts/frame-runs` against the 15.6 ms a tick has at 64 Hz.

What was expected is in the Outcome below, next to what was measured.

## Known loose ends, declared rather than discovered

- **No corner stairs.** A convex or concave corner gets a slab or a plain cube.
  A corner Shape needs neighbour-dependent geometry (ADR-0006: not
  expressible).
- **Two-Voxel cliffs and taller are not eased.** A tall Shape or a multi-step
  ramp would be needed, and neither is on the table.
- **The Material is always Grass**, because the topmost cube of an uncarved
  column is. A snow line or a beach is a Mod's decision once Materials are
  declared (step 7).
- **The generator's output is a compatibility surface** again, as it was for
  caves. Nothing persists chunks yet; when saves arrive, the rule, the default
  `min_rise_m` and the orientation mapping are as pinned as the terrain's.
- **`min_rise_m` is a placeholder for the controller's step height.**

## Done when

`cargo dev` shows ramps on real hills with light and occlusion looking right (a
screenshot from the capture is taken and shown); `./scripts/cargo-ci` passes;
`cargo bench -p tesserae-core` is compared to the saved baseline with
before/after numbers in the PR, including `worldgen/straddling` and the
streaming worst tick; the cap sweep is a test; `terrain_output_is_pinned`
passes untouched.

## Outcome

Built as decided, with these corrections, recorded where they happened.

- **A Chunk edge is computed from the Chunk that owns it** (`axis_m`), after
  review: the other spelling differs in the last bit at Voxel sizes that are not
  a power of two, and a stair on an edge is decided about a column in the next
  Chunk. Pinned at 0.3 m.
- **The surface border is sampled only when smoothing runs**, so `smoothing:
  None` is free.
- **The const assertion tying the cap to the column count was dropped**: the
  enforced budget already bounds the pass below the cap.
- **The first baseline was discarded**: three sessions were benchmarking at once.
  The numbers are `main` built in its own worktree against this branch, run
  alternately.

What it costs (ticket 04, 05), at 1 m Voxels, per surface Chunk:

| Stage    | Before   | After    | Why                                       |
| -------- | -------- | -------- | ----------------------------------------- |
| Generate | 112 us   | 178 us   | the pass, and palette growth              |
| Light    | 98 us    | 176 us   | the shaped flood, which any ramp selects  |
| Mesh     | 381 us   | 411 us   | pass 2 at ~170 ns a stair                 |

The densest Chunk of a 400-Chunk sweep holds 454 shaped Voxels (budget 512, cap
1024) and takes 457 us to mesh. The streaming budget did not move (worst tick
1.6 to 1.8 ms at load 20; mesh upload 11 to 16 us).

Loose ends, declared rather than discovered:

- **The shaped light flood is the cost to attack**, not the generator: it nearly
  doubles relight for every surface Chunk with a ramp. Only a Chunk that has
  one pays.
- **A World with Voxels under `min_rise_m` is not smoothed**, by design, and the
  threshold is a placeholder for the controller's step height.
- **The Material is always Grass**, and there are no corner stairs.
- **Not mutation-tested**: the real-terrain hole check and the two ramp light
  tests were not shown failing against a deliberately broken mesher or solver.
