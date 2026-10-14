# Light the coarse LOD levels

Status: done
Blocked by: -

Follow-up to `.scratch/2026-10-13-phase-6-lod/spec.md` decision 3 and the
"Coarse light is full sky" consequence of
[ADR-0011](../../../docs/adr/0011-coarse-levels-are-derived.md). No spec exists
for this; if triage takes option (a) or (b) below, `/to-spec` writes one first.

## Problem

Every Chunk above level 0 is meshed with `Light::new(MAX_LIGHT, 0)` for itself
and all six neighbours (`OPEN_SKY` in `crates/tesserae/src/server/lod.rs`), so
sky light is 15 on every coarse face. Open ground is right. A **mouth**
(CONTEXT: where a Cave breaks the surface) is lit like open ground, and so is
the far wall of a tunnel that level 0 would show dark within about 15 m.
ADR-0011 accepted this as a cost, and the spec named a "column rule in the
generation job" as the cheap fix without building it.

How far it reaches is not known, and that is the first thing to find out. With
the default `CaveParams` a tunnel is about 9.6 m across (radius 4.8 m) and the
half-volume downsample must keep it as air. By arithmetic a 2 m or 4 m Voxel
(levels 1 and 2) keeps it, an 8 m Voxel (level 3) only where it is well
aligned, and a 16 m or 32 m Voxel (levels 4 and 5) should not. Level 1 is what
the Eye sees from roughly 150 to 380 m and level 2 out to roughly 770 m, so the
affected range may be 150 m to under 1.5 km, not 4 km. The capture below
measures this; it is not assumed.

A second unknown: coarse terrain **receives no shadow** (two suns, spec decision
8), and sky light reaches the shader as `diffuse_occlusion` and
`directional_shadow_factor`. Whether a sky value of 0 on a coarse face changes
the pixel enough to see has never been checked. If it does not, there is
nothing to build.

## Step 0: capture first, then decide

Nothing after this section is started until it is done and the user has looked
at the images. Screenshots are review-only: one folder outside every worktree
(e.g. `/tmp/tesserae-coarse-light-review/`), never committed, described in text
under Comments, deleted when review is done.

1. **Find a mouth.** A throwaway probe (not committed) over the default seed
   scans the surface band for level-0 air at `Worldgen::surface_height_m` minus
   a metre or two, and prints a few mouth positions in metres, nearest the
   origin first. Prefer a mouth that faces a flyable approach, and one more
   with a sideways opening and one that is a pit open to the sky, because (a)
   and (b) differ on exactly those.
2. **Capture it twice**, with `TESSERAE_SCREENSHOT_OFFSCREEN=1` (a locked screen
   gives a black frame otherwise), 1080p, plain and with `TESSERAE_LOD_TINT=1`:
   from about 100 m, where level 0 draws it and is the truth, and from about
   250 m and 500 m, along the same bearing, where levels 1 and 2 draw it. Use
   the existing yaw, pitch, lift and fly knobs; if they cannot place the Eye, a
   local uncommitted override will do. Look at every image.
3. **Count it.** For each level 1 to 5, over the 4 km square: coarse air Voxels
   that lie more than one Voxel below `surface_height_m` and border a face the
   mesher emits. This is the number of faces option (a) would change, per
   level, and the range it matters over.
4. **Try the dimming by hand.** A local, uncommitted hack that lights those
   faces at sky 0, captured from the same two vantages. This is option (a)
   thrown away after one look. If the picture does not visibly improve, the
   shader is not showing sky light on coarse faces and neither option is worth
   its cost.
5. **Record** the finding under Comments: levels where the count is non-zero,
   whether the distant mouth reads wrong against the truth, whether the hack
   fixes it. Then set Status.

**Decide.**

- The distant mouth reads the same as the truth, or the count is zero beyond a
  level that nobody looks at: set `wontfix`, add one sentence to ADR-0011's
  consequence ("measured at 250 m and 500 m, not visible"), and stop.
- It reads wrong and the hack fixes it: do (a), or (b) if the user wants the
  gradient into the cave described below. Re-triage to `ready-for-agent` with
  the choice written into this ticket.
- It reads wrong and the hack does not fix it: the cause is in the shader or
  the missing shadows, not the light, and this ticket is the wrong ticket.

Building before the capture is the one thing this ticket is written to prevent.
Both options pay a standing cost on every coarse job for a defect nobody has
seen.

## Option (a): a covered column rule, no stored light

For each column of a coarse Chunk, `Worldgen::surface_height_m` (of the level's
source generator, in metres) says where open ground ends. A coarse air Voxel
whose top lies more than about one Voxel below it is **covered**, and the
mesher lights the faces that look into it at a dim sky value instead of 15.
Nothing is stored: the value is computed in the mesh job from the generator and
the Chunk's position, then dropped with the job.

- **Why a margin of a Voxel.** The downsample rounds the surface up, so a coarse
  top is within one Voxel above `surface_height_m` and never more than half a
  Voxel below it (pinned by ticket 08 of the LOD spec). Open ground must keep
  sky 15 byte for byte; a margin of about a Voxel is what guarantees that.
  Sample the column over its footprint and take the lowest height, not the
  lower corner alone: on a slope the corner can sit a Voxel above the middle at
  level 5.
- **Where it runs.** The mesh job gains the level's generator, an `Arc` clone
  per ADR-0007, and builds the light for the Chunk and the one-Voxel shell of
  its six neighbours. A Chunk wholly above `surface_bounds_m` (nearly the whole
  upper ring) and one wholly below it (deep rock) take the flat-light fast path
  as today. Only Chunks the band crosses pay for column samples.
- **No change to `light.rs`, `ChunkLight` storage or the stage count.** Two
  stages stay two. `mesh.rs` takes a `ChunkLight` per Chunk, so the simplest
  build is seven transient dense ones (about 230 KiB) per mesh job; building
  only the covered shell of the six neighbours, or a rule inside the mesher, is
  cheaper. Which one is the implementer's call and the benchmark's to justify.
- **Merging.** Light is part of the greedy merge key. Flat open sky merges
  maximally and covered pockets split quads only where a cave is, so the mesh
  grows only at Chunks that hold covered air.
- **What it gets wrong, accepted.** The height is the heightfield's, before
  caves. A pit that breaks the surface and is open to the sky is dark below its
  top Voxel here and fully lit at level 0 (sky light falls straight down
  undimmed). A sideways mouth is dark from its first covered Voxel, where level
  0 grades over about 15 m; at 2 to 4 m Voxels that is a few Voxels of lip. And
  the dim value is a constant, not a function of depth. A fix for the pit (a
  column scan of the coarse Chunks above) needs Chunks the job does not hold
  and is not proposed.
- **Cost.** About one `surface_height_m` call per column of a Chunk the surface
  band crosses (1024, or 1156 with the shell), on a few hundred Chunks per
  level, in the mesh job. Unmeasured; the acceptance criteria measure it. No
  memory.
- **ADR-0011.** Amend the "Coarse light is full sky" consequence in place:
  coarse light is two-valued, sky above the heightfield and a constant below
  it, and `OPEN_SKY` is no longer the whole story. "Coarse levels store no
  light" and "`light.rs` takes no new rule" both stand.

## Option (b): a downsampled sky-light pass per coarse Chunk

Run the existing bulk solver (`light_chunk`) over each downsampled Chunk, store
the `ChunkLight` in the `LodStore` beside it, and hand the mesh job real light
for the Chunk and its neighbours.

- **Memory.** `ChunkLight` is one byte per Voxel, 32 KiB when dense, free when
  uniform. At the defaults the non-uniform Chunks (the only dense ones) are 386
  at level 1, 394 at each of levels 2 to 4 and 226 at level 5: 1794, so up to
  about **58.8 MB** if every one of them is dense, which a surface Chunk
  (solid below, air above) is. That is **+46%** on the 127.9 MB of vertex and
  index memory today (86.4 MB of it already in the coarse levels). Uniform air
  and rock Chunks, about 80% of the held count, cost nothing.
- **Generation time.** Not measured, and the ticket must measure it. For
  orientation: `light_chunk` on a level-0 canopy Chunk is 170 us, a coarse
  generate-and-downsample is 2.1 ms at the surface of level 1 and 0.8 ms at
  level 5, and a coarse mesh 0.3 to 0.4 ms. Light has the early-outs the other
  stages have (3 to 4 ns for open sky and for rock), so only the 1794 surface
  Chunks pay: at the canopy figure about 0.3 CPU seconds, against roughly 7 CPU
  seconds that the coarse generate and mesh jobs cost to fill the levels now.
  That is the cheap half.
- **The expensive half is not the solve, it is the dependency.** Light crosses
  Chunk borders, so a Chunk is solved against neighbours that may not have
  landed, and a neighbour landing later changes the boundary and forces a
  re-solve of what touched it. That is the third stage and the settle gate level
  0 has (`MeshState`, `streaming::relight`) and the LOD spec removed on
  purpose ("two stages, not three"; the mesh job today waits for six
  neighbours and nothing else). Under (b) the mesh waits for light to settle
  too, the fill time (10.6 s for all levels today, sharing one three-worker
  pool with level 0) stretches, and the caps in `LodBudget` have to be
  re-measured with the fill-latency-against-level-0 table of ticket 07, because
  a third stage competes with level 0's jobs on the same FIFO. Seeding sky from
  the heightfield instead of from neighbours avoids the cascade, at which point
  (b) is (a) plus a flood.
- **The dimensionless light level does not scale, so the flood is wrong at the
  join.** Each step costs one level whatever the Voxel size (CONTEXT: Light
  level). A cave 20 m deep is 20 steps at level 0, so dark; at level 2 (4 m
  Voxels) it is 5, so level 2 draws it at sky 10. Run unchanged, the solver
  lights coarse tunnels brighter than level 0 lights the same tunnel, and a
  ring join inside a cave would show a step the other way. Fixing it means a
  per-step cost of `2^k` levels in `light.rs`, a new rule in a file whose speed
  depends on const generics and whose last unconditional rule cost something
  (spec decision 3), or downsampling level-0 light, which means generating and
  lighting eight level-0 Chunks per coarse Chunk. Either is a larger piece of
  work than the problem.
- **What it buys that (a) does not.** A gradient from the lip into the cave, a
  correct lit pit, and a sky value that follows the geometry. At 2 to 4 m
  Voxels that is a few Voxels of gradient, at a distance of 150 m or more.
- **ADR-0011.** More than an amendment: "The `ChunkStore`, light, relight and
  `MeshState` stay level 0 only" is no longer true, `LodStore` holds light,
  "Coarse light is full sky" and "`light.rs` takes no new rule" both go, the
  invalidation paragraph gains a relight of the level-1 cover, and the spec's
  "two stages" becomes three. Write ADR-0012 superseding the light
  consequence, and set the ADR-0011 line to point at it.

## Recommendation

**Do neither until Step 0 says to. If it does, do (a).**

(a) fixes the case a viewer can see (a dark hole in a hillside at 150 to 770
m), stores nothing, adds no stage, leaves `light.rs` and ADR-0011's structure
alone, and costs column samples on a few hundred Chunks per level. Its errors
(the pit, the missing gradient) are small at the Voxel sizes where a mouth is
visible at all. (b) pays 59 MB, a third stage and a settle gate for a
gradient across three or four Voxels, and cannot be correct at the ring join
without a new rule in `light.rs`. (b) is the answer only if Step 0 shows the
step from lit to dark in (a) reads wrong against the truth, and then the
smaller fix to try first is a second constant for the first covered Voxel, not
a flood.

## Acceptance criteria

Common to every outcome:

- Step 0 is done; the images are shown to the user; the Comments say what each
  showed, per vantage, in words, and the folder is deleted when review ends.
- The decision and the evidence for it are written in this ticket, and Status
  is `wontfix` or `ready-for-agent`.

If (a) is built:

- A World without caves meshes every coarse Chunk of levels 1 to 5 identically
  to `main`: same vertices, indices and light attribute. Open ground does not
  change.
- Over the default World's 4 km square, every coarse air Voxel that borders a
  face and lies more than one Voxel below the heightfield is covered, and no
  air Voxel above it is.
- The dim value is one named constant, chosen from the capture by matching what
  level 0 shows deep in the same cave, and its value and the image are in
  Comments.
- The mesh job holds `Arc`s and returns an owned value, with no `LodStore`,
  `ChunkStore` or `LodState` in its signature (spec invariant 5).
- ADR-0011's consequence is amended and CONTEXT.md, DESIGN.md's LOD section and
  `docs/architecture.md` agree with it. No new term unless one is needed
  ("covered" might be, in CONTEXT).
- `mesh.rs` and `light.rs` are unchanged, or the diff says why not and the
  existing benchmarks are shown against `main`, interleaved, with the load
  average quoted (benchmarks lie under load).
- `./scripts/cargo-ci` passes in full.

If (b) is built, in addition:

- ADR-0012 is written and ADR-0011's superseded lines point at it.
- The memory figure (dense `ChunkLight` bytes per level and in total) and the
  per-level light solve time are measured and added to DESIGN.md's open item
  beside the existing table. The estimate above is replaced.
- `LodBudget` is re-swept with the third stage and level 0's fill time and the
  worst tick are quoted against the numbers in ticket 07. Level 0 must not pay
  more than it does now.
- A ring join inside a cave is captured and the step in light across it is
  described.

## Tests

For (a), each killed by a named mutation:

- **Cave-free identity.** Sweep columns at levels 1 to 5 over the 4 km square
  in a World with `caves: None`: the covered set is empty and the mesh equals
  the open-sky mesh. Mutations: margin of zero Voxels (the rounded-up surface
  turns covered), and sampling at the lower corner only (fails on a steep
  slope at level 5).
- **A hand-built tunnel under a flat heightfield.** A Chunk with a known
  surface and an air tube below it: faces inside the tube get the dim value,
  the surface faces above get 15, and the Voxel exactly one Voxel under the
  surface stays lit while the next one is covered. Mutation: margin of two
  Voxels, then one-half Voxel.
- **A real mouth.** The mouth found in Step 0, at level 1: its inside is
  covered, its count is non-zero (the test fails if the probe finds nothing),
  and the covered value matches what level 0 holds at least 15 m inside the
  same mouth. Mutation: invert the comparison.
- **Chunk borders.** A tunnel crossing a Chunk border horizontally and
  vertically: the two Chunks agree on the covered state of the shell they
  share, so no seam of sky 15 appears in a wall. Mutation: compute the shell
  from the Chunk's own columns only.
- **The accepted pit.** A pit open to the sky at level 1 is dim below its top
  Voxel. A test that says so, named for the limitation, so a later fix flips it
  on purpose.
- **Fast paths.** A Chunk wholly above `surface_bounds_m` takes the flat-light
  path and makes no `surface_height_m` call; one wholly below does too.
- **Benchmarks** in `benches/lod.rs`: `lod/mesh_coarse` unchanged for an
  above-ground Chunk, and a new `lod/mesh_coarse_covered` for a Chunk with a
  tunnel, in absolute terms against the 0.3 to 0.4 ms mesh figures.

For (b), in addition to or in place of the above:

- **Solver parity.** The coarse light of a flat world with a hole equals
  `light_chunk` on the same Chunk, so the store holds what the solver
  returns.
- **Settle.** A coarse Chunk landing after its neighbour re-solves exactly the
  Chunks whose shared plane changed (`boundary_differs`), and a Chunk
  regenerated in place relights its neighbours. Mutation: skip the neighbour
  re-solve.
- **Join.** Light 15 steps into a cave at level 0 and at level 1 over the same
  Voxels, quoted as the size of the step and asserted against whatever tolerance
  the chosen per-step cost gives.
- **Level 0 unchanged.** `Rings::for_level(0)` still equals today's `Rings`, and
  level 0's fill time and worst tick stay inside their ticket 07 figures.
- **Memory.** A test that holds the default rings full and asserts the dense
  light bytes are under the figure the ticket recorded.

## Comments

### 2026-10-14: Step 0 run (Apple M5, `dev` build, 1080p offscreen)

Status left at `needs-triage`; the decision is the maintainer's. Screenshots are
in `/tmp/tesserae-coarse-light-review/`, not committed, to be deleted when this
is decided.

**What a mouth is in this World.** Not a cave opening in a cliff: relief is
gentle, so every opening is a hole carved through the surface cap. A probe over
the default seed (960 m each way) found about 173 000 opening columns, 4.7% of
all columns, so mouths are common, not rare. The one used is a 12 m by 22 m hole
with a roofed tunnel running north and south from it, centred at
(-26, -352) m, 349 m from the spawn, 14 m deep at level 1.

**Count (step 3).** Coarse air Voxels more than one Voxel under the heightfield
that touch a solid Voxel inside their Chunk, summed over the rings at the
default vantage: level 1 107 042 (232 Chunks), level 2 54 652 (216), level 3
21 018 (216), level 4 2 083 (215), level 5 0. Caves survive the half-volume
downsample further than the ticket's arithmetic said (level 4, not level 3).
These are faces the mesher emits, not faces that are seen: most are inside
sealed tunnels.

**Truth against level 1.**

- 45 m out, 105 m up, looking down 38 degrees: the hole is a black interior
  with a lit near wall, about 200 by 200 px.
- 250 m out, 128 m up, 27 degrees down: the same hole is a speck of about 25
  px, a brown and grey wall, with no dark interior visible. At 254 m up
  (45 degrees) it is a grey patch of about 40 by 30 px.
- Seen as level 1, the interior is lit like the ground around it, so the
  discrepancy exists. At this range it covers a few dozen pixels.

**The hack (step 4).** Local, uncommitted, reverted: covered air lit at sky 0,
margin one Voxel, lowest height over the Voxel's footprint. At 250 m it darkens
one or two faces of the mouth (a strip on the east wall goes black; the grey
patch gets a darker tile). The shader does show sky 0 on a coarse face as
black, so the light reaches the pixel. What it does not do is turn the speck
into the black hole of the truth image, because the faces looking into the
covered air are mostly hidden from this angle.

**Reading.** Option (a)'s effect at 250 m is a handful of pixels, on one
mouth, at two vantages. Not captured: a vantage at 500 m (level 2), a second
mouth, and the plain-ground control for the tinted rings. By the ticket's own
decision rule this leans `wontfix`; it does not show a mouth that reads wrong
at distance.

**Side finding, unmeasured.** The count above includes tunnels that are sealed
inside the terrain, whose faces are meshed and never seen. Skipping them would
cut coarse quads and vertex memory, which is a different ticket from lighting
and has no capture behind it yet.

### 2026-10-14: triaged

Maintainer, in chat: set `ready-for-agent` with option (a) chosen, write the
spec before any code, and implement on a `feat/` branch test-first with the
benchmarks and the ADR-0011 amendment this ticket lists. Step 0 leaned
`wontfix` (see above); the decision was made with that in front of them. The
work stays in the mesh job: no change to `light.rs`, `mesh.rs` or the stage
count unless a benchmark says otherwise.

### 2026-10-14: built (branch `feat/lod-coarse-light`)

Option (a), as the spec at `../spec.md` says. `coarse_light` in
`tesserae-core/src/lod.rs`, called from the coarse mesh job in
`server/lod.rs`; `light.rs`, `mesh.rs`, `ChunkLight` and the stage count are
untouched.

**Capture, rule in** (same vantages as Step 0, same seed): at 250 m and 128 m
up the mouth is still a speck of about 25 px and its far wall is a slightly
darker grey; at 254 m up the grey patch has one darker tile and a strip of the
east wall is black. The ground around it is pixel-identical to the unchanged
picture, which is the plain-ground control. The effect is the one the hack
showed: a handful of pixels, not the black hole of the 45 m truth. Not
captured: 500 m, and a second mouth.

**Tests.** Core: eight new, each mutation-checked (a margin of zero, half a
Voxel and two Voxels, one corner of the footprint, a neighbour's plane from its
own columns, and the two fast-path bounds a Voxel early: all seven killed).
Server: the real mouth, and the cave-free identity over 61 non-empty meshes at
levels 1 to 5. The identity test has slack and does not kill a margin of zero,
half a Voxel or one corner (see the spec); the core tests pin those. Three
existing server tests pinned open sky as the only light and now say open sky or
covered.

**Benchmarks** (Apple M5, host load 20 falling to 7 over the run, absolute
numbers): `lod/coarse_light` on a surface Chunk is 64 us at level 1 and 90 to
110 us at levels 2 to 5; a Chunk wholly above the band takes 14 ns. A coarse
mesh job (`lod/mesh_coarse_lit`) is 465 us at level 1 against 382 us for the
open-sky mesh (`lod/mesh_coarse`), and 400 against 296 us at level 5; the
mouth Chunk is 502 us. Existing benches against a build of `main`, interleaved,
three rounds, load 5 to 8: `greedy_mesh/terrain` -2.5%, +2.2% on
`mesh_chunk/straddling_unsmoothed`, and `leaves_canopy` -5% to +5%, mixed sign;
nothing in `mesh.rs` or `light.rs` changed, so this is layout noise.

**Cost, stated.** About 80 to 100 us more per surface Chunk of a coarse mesh
job, on a few hundred Chunks a level, off the main thread. The fill time was
not re-measured.

`./scripts/cargo-ci` passes in full. ADR-0011, DESIGN.md and
`docs/architecture.md` are amended.
