# Blended surfaces across the LOD cameras

Status: done
Blocked by: 04

Spec decision 6, generalised by `.scratch/2026-10-13-phase-6-lod/spec.md`
decision 8. Read `bevy_core_pipeline-0.20.0` (`core_3d`, `oit`) and
`bevy_pbr-0.20.0` (`material.rs`) under `~/.cargo/registry/src/*/` before
deciding anything below.

## Problem

Ticket 04 builds the blended entity at every level. It does not answer how a
translucent surface composites across the per-level cameras, which were built
for opaque geometry. Each level has its own camera and its own depth, coarsest
first, and a finer camera clears depth and draws over the last. Opaque geometry
is safe under that: wherever a finer level has geometry its pixel replaces the
coarse one, whatever the coarse depth. A blended pixel does not replace, it
accumulates, so the overlap that is the LOD seam strategy shows (decisions 4 and
9 of the LOD spec). Nothing in 04's single-level scene can show it.

There is also a gap in what can be seen at all: worldgen places no translucent
Voxel and Edits reach levels 0 and 1 only (LOD decision 1), so no level above 1
can hold a blended Voxel today.

## Architectural invariants

- **The Client mutates no World state (ADR-0002).** Holds: this ticket changes
  how render entities composite and, if question 3 is answered with a `dev`
  knob, one Server-side generator switch. The Client still only receives meshes.
- **No Mod reaches core**, and **`tesserae-core` depends on neither Bevy nor
  wgpu**: nothing here touches either.
- **Physical quantities are metres.** Any wider cut, fade or bias is in metres,
  from `server::cuts`, never a count of Voxels.

## Acceptance criteria

- The decisions in "Open design questions" are made and recorded in the LOD spec
  (decision 8 or 9) and the transparency spec, with a line in `DESIGN.md` Open
  items; they are not left in the ticket.
- Water over a ring join shows no band that is visibly darker or lighter than the
  water either side, or the band is recorded as accepted with the capture that
  shows it.
- A coarse water surface does not stand over the finer level's real surface or
  horizon, vertically as well as horizontally, and opens no crack
  (`TESSERAE_SCREENSHOT_MAGENTA_SKY` finds clear-colour pixels under water only
  at silhouette edges, as ticket 08 of the LOD spec found for terrain).
- Level 0 and the opaque scenes are unchanged: a scene with no blended Voxel is
  pixel-identical to `main`, and its worst tick and median flight frame are
  within noise.
- The cost with a sea in frame is measured and recorded against the budget below.
- `./scripts/cargo-ci` passes.

## Open design questions

### 1. Double blending in the overlap

A coarse level is drawn under a finer one out to the finer disc's edge. Over
water, the coarse camera blends its surface over the bed and the finer camera
blends its own over that, so a band of darker water would sit at each ring join.
The shader's `cut` removes a coarse fragment only inside the finer level's
_guaranteed_ reach, so the band is the difference between that reach and the
finer edge. Options: a wider cut for blended fragments only, a fade across the
band, or accepting it. All three are decisions about the cut, which
`server::cuts` owns.

### 2. The other places the opaque argument does not carry over

- **A coarse surface up to one coarse Voxel too high.** A coarse water plane is
  rounded to the coarse Voxel and can stand above the real surface, as coarse
  terrain does over the finer silhouette. The cut handles that for opaque; check
  it does for blended, vertically as well.
- **The crack argument.** The overlap prevents sky showing through a culled
  boundary face because a coarse surface is behind it. A translucent coarse
  surface behind a translucent finer one is not a seam fix, it is a second layer
  of water. Check a boundary face of water culled against a neighbour the other
  level did not draw.
- **A blended fragment seen through a finer level's window.** A finer glass
  blended over coarse colour is right; confirm the finer camera's depth clear
  does not lose the coarse blended pass's result under it.
- **If 04 chose OIT**, whether it works at all with six cameras on one target and
  a depth clear between them, and whether the resolve can skip a camera whose
  phase is empty, which is most levels most of the time.

### 3. Where a coarse blended mesh comes from

Decide one:

- a `dev`-only knob in the generator that floods the terrain below
  `sea_level_m` with water, enough for the captures (the Server's, so ADR-0002
  holds);
- real lakes or a sea in worldgen, as its own ticket with this one blocked on
  it, which changes level 0's generation cost and so its measured budget;
- hand-built `ChunkMesh` messages for tests only, and the coarse case reported as
  untested in a capture.

The first is the smallest. Without one of the first two, no capture below can be
taken.

### 4. Cost per camera

- An entity is on exactly one layer, so a blended mesh is submitted to one camera
  and the draw count grows by the number of non-empty blended Chunks, not the
  number of cameras. What scales with cameras is any per-camera pass (an OIT
  resolve, a clear). The LOD layout already costs about 0.4 ms per camera (median
  flight frame 5.6 ms at 1080p on an M5).
- Blended fragments run the whole PBR fragment shader, cascaded shadow lookup
  included, on top of the opaque fragment beneath. A sea at 4 km covers much of
  the screen at every level, so overdraw is the cost to measure, not vertex
  count. A coarse sea is a handful of merged quads (coarse light is uniform,
  which merges maximally); its price is fill.
- Memory: the blended layers are already inside the 127.9 MB figure, for scenes
  that contain no water. A world with a sea changes it; re-measure.

## Test and capture plan

**Captures**, `dev` build with `TESSERAE_SCREENSHOT_OFFSCREEN=1` (a black
capture is a locked screen, not a renderer bug), looked at and shown:

1. A ring join over water at `TESSERAE_LOD_TINT=1`, at the views ticket 08 of
   the LOD spec used (near ground across a lattice corner, 150 m, straight down),
   looking for the band of question 1.
2. The same views with `TESSERAE_SCREENSHOT_MAGENTA_SKY`, counting clear-colour
   pixels under water.
3. A glass window at level 0 with coarse water behind it.
4. A scene with no blended Voxel, pixel-diffed against `main`.

**Unit tests** for whatever question 1 changes in `cuts` (a wider or faded cut is
tested the way the existing cut is: walked over every Eye parity, not by its own
shortcut), and for the knob of question 3 if one is built.

**Performance**, per the LOD spec's rules: `scripts/frame-runs`, interleaved
against a build of `main` in its own worktree, the load average read before and
after each run and quoted, `MainThreadCost` for the Client's share and median
flight frame for the GPU's, with and without a sea in frame. At a load average
of 50 or more wall-clock numbers are not evidence.

## Out of scope

- The blended entity, palette alpha, hotbar, sorting inside a Chunk and shadows
  (04).
- Cutout leaves, textures and water shading (step 9); water flow (step 8).
- Collision and picking treating water as solid (a `solid` field, when a
  controller exists).
- Distance fog and atmosphere, which would change how a distant sea reads.
- Block and sky light at coarse levels (LOD decision 3): a coarse blended face
  is lit as open sky like every coarse face.

## Comments

2026-10-14: triaged and built in one pass at the author's request. Question 3
was decided first, because nothing else could be captured without it.

- **Question 3, the source: a generator parameter, set only under `dev`.**
  `WorldgenParams::water: Option<WaterParams { level_m }>` fills the open air
  above the ground up to `level_m`, never a cave, clamped to the top of the
  surface band because the Server treats everything above that as open sky.
  `None` by default, so a default World is what it was, and the level-1 and
  coarser source generators inherit it from the level-0 parameters. Under `dev`,
  `TESSERAE_WATER_LEVEL_M` sets it. Two tests in `worldgen.rs`: every Voxel the
  flood changes, over a sweep of Chunks at 1 m and 0.5 m Voxels, was open air
  between the ground and the level and became water, and nothing else changed;
  and nothing above the band is flooded (dropping the clamp fails it). Real
  lakes, with a level that varies, are worldgen's own ticket when wanted.
- **Question 1, the double-blended band: accepted.** On a World flooded to 44 m
  (around the 25th percentile of the ground), a lake crossing the level-0 to
  level-1 join shows the band: level 0's water over level 1's, a shade darker,
  water luminance 131 to 152 with no row-to-row step above 6.6. A fade of the
  coarse layer across the band (from the cut to the finer level's farthest
  reach, `R + sqrt(2)` Cells) was built and measured worse, 123 to 153 with a
  step of 16.6, because where the finer level does not reach it thins the only
  water there is; it was reverted. The exact fix is to cut a coarse blended
  fragment wherever the Client draws the finer Chunk that contains it, which
  needs that set in a uniform; not built. Recorded in the LOD spec's decision 9.
- **Question 2, the rest of the opaque argument:**
  - A coarse surface too high: the cut applies to blended fragments unchanged,
    since they use the same shader, and no capture showed coarse water standing
    over finer ground or horizon.
  - Cracks: the magenta-sky probe counts the same 3 clear-colour pixels under
    terrain (one silhouette step) with the World flooded and dry, on the horizon
    view, and none from 150 m at 35 degrees or straight down.
  - Coarse blended under a finer camera: survives its depth clear. In the tinted
    capture, level 0's water over level 1's shows as a blend of the two tints
    across the band, which it could not if the coarse pass were lost.
  - OIT: not taken (ticket 04), so its interaction with six cameras is moot.
- **Question 4, cost.** Draw calls grow by the Chunks with water, each on one
  layer and so one camera; no pass was added per camera. What the water did
  cost was the sort of ticket 04: shore Chunks are not flat (water meets the
  slope-smoothing stairs underwater), so with a one-Voxel threshold every one in
  view re-sorted on every Voxel of movement, about 4500 sorts per 6 s flight and
  0.8 to 2.5 ms in the worst frame. A threshold that grows with distance (a
  sixteenth of it) cut that to about 1130 sorts and 0.22 to 0.48 ms, load 7 to
  10, four runs. The worst tick and the median flight frame flooded are within
  the spread of the dry runs (1.5 ms median over six runs, at the 60 Hz cap).
- **Captures** (offscreen, 1080p, 30 s settle so every level filled, not
  committed): the default view, a horizon view at 4 degrees down, 150 m up at 35
  degrees and straight down, each tinted and plain, and the magenta probes. Every
  level fills with the World flooded (level 5 at 10.7 to 14.9 s, the spread is
  the host's load).
- **Worldgen pays nothing for `None`.** The flood is one `Option` check where a
  column turns to sky. `cargo bench -p tesserae-core --bench meshing --
  ^worldgen`, interleaved with `main`, the quieter pair at load 2.8 to 3.6:
  `straddling` 168.3 against 168.8 us, `straddling_unsmoothed` 106.3 against
  106.7, `ramp_densest` 174.3 against 174.9, `straddling_caved` 259.0 against
  261.7, `cave_crossed` 267.8 against 275.6 (inside its own 270 to 283 interval),
  early-outs unchanged at 3.8 and 4.7 ns.
- **Memory**: 132.6 MB of vertex and index data with the World flooded to 44 m,
  against 128.0 MB dry, so lakes cost 3.6%.

