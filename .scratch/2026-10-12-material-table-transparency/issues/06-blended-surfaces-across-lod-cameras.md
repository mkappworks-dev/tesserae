# Blended surfaces across the LOD cameras

Status: needs-triage
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
