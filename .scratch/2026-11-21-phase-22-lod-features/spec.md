# LOD features: coarse levels that agree with level 0, and a longer reach

Status: ready-for-agent. Decisions Q1 to Q23 were settled in a grilling round on
2026-11-21.

Source: branch `feat/phase-22-lod-features`, cut from `main` at `65037cdd`.
DESIGN.md step 22. Depends on steps 6, 18 and 20. Builds on
`.scratch/2026-10-13-phase-6-lod/` (the rings and the per-level budget),
`.scratch/2026-10-14-lod-coarse-light/`,
`.scratch/2026-10-14-coarse-lod-durable-edits/` and
`.scratch/2026-11-20-render-shadows-and-culling/` (ticket 07 and its gate
method). Decisions: [ADR-0035](../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md),
[ADR-0036](../../docs/adr/0036-the-quality-curve-is-a-player-preference-the-server-clamps.md).
Tickets: [issues/](issues/).

## Problem Statement

Flying toward distant terrain, shores turn from sand to sea and back at each
ring join, and trees appear. Beyond 192 m nothing stands on the ground. The rings
also switch to coarser Voxels about half as far out as other engines with
distant terrain, and end at 4 km.

What the code does today, measured on the default Terra seed (six 512 m
windows holding coast):

- **Each coarse level moves the coast by its own rounding.** A seabed is land
  from 23 m at level 0, about 23 m at level 1, 22 m at level 2, 20 m at level
  3, 8 m at level 4 and 16 m at level 5. Of coastal coarse columns, 10%, 39%,
  40%, 35% and 31% at levels 1 to 5 are dry where level 0 is wet at their
  centre; the top Material differs on 1% to 18% of dry ones.
- **Level 4 loses the shallow sea.** The waterline tie in `server/lod.rs` reads
  `water.level_m`, which `Worldgen::new` has already overwritten with
  `sea_level_m`, so the tie is never broken. A 16 m Voxel's middle is exactly
  Terra's 24 m sea, and that Voxel never holds water: the sea is drawn as ground
  8 m under the water's surface.
- **A coarse column takes its Biome at its corner**, so seabed made land keeps
  the ocean Biome and shows as sand.
- **Trees and Structures stop at level 0.** Coarse sources are the bare
  generator (`without_surface_features`, no Templates). Level 1 takes held
  level-0 Chunks as children, so it has trees exactly where those Chunks
  happened to be held when its job ran, and is never redone.
- **Rings double from the view distance**: level k ends at
  `view_distance_m * 2^k`. The engine's defaults are 192 m and 4096 m; Terra
  declares 160 m and 3000 m, so on Terra level k ends at `160 * 2^k` m, to
  3000 m in six levels.
- **Level 1's source shares level 0's column cache**, which holds about 1 km² of
  1 m columns. A wider level 1 would evict level 0's own columns.

## Solution

Tickets in this order. To save wall clock: 02, 06 and 07 start beside 01; the
full timing and GPU series run only at 01 (in a quiet window) and 09, other
tickets reporting coarse-Chunk CPU time from an interleaved micro-benchmark;
captures are reviewed in two batches, after 05 and at 09.

1. **Baseline.** The agreement measurement as a test (failing levels recorded,
   not asserted), and the budget series on `origin/main`: per-level Chunks,
   vertex memory, fill time, timing and GPU, with the shadows comparisons.
2. **Coarse agreement** at every coarse level, which also ends level 4's lost
   sea.
3. **Tree Impostors at levels 1 and 2**, from a second level-0 generator that
   answers draws only, and none over saved changes.
4. **Structure Impostors** at every coarse level, Settlements and their paths
   included.
5. **Canopy cover** from level 3 up.
6. **Coarsest level first** in the fill order.
7. **The Quality curve** as a table the Server streams from, per Player,
   clamped to the Game's values; holes and overlaps derived per pair.
8. **A Distance settings tab** for it: the mockup page (built during the grilling,
   approved before this ticket), then the Client page.
9. **The default curve**, about `384 * 2^k` m to 8192 m, with lighter fog, only
   once the budgets pass and you approve the captures.
10. **Ground tint** from the plants a column would hold: lands only if it
    improves the look and stays in budget.

## User Stories

1. As a player, I want a coast to stay where it is as I fly toward it, so that
   the far field reads as the same World.
2. As a player, I want forests, buildings and villages on the horizon, so that
   a view from a hill shows where to go.
3. As a player, I want a tree I felled to stay felled when I look back from far
   away, so that distance does not undo my work.
4. As a player, I want to see as far as other engines with distant terrain
   show, so that the World feels large.
5. As a player on a weaker machine, I want a lower preset, so that the frame
   rate holds.
6. As a developer, I want each coarse level's agreement with level 0 measured,
   so that a regression fails a test rather than a flight.
7. As a Game author, I want my view values to stay the ceiling, so that a Game
   sets what its Server can afford.

## Implementation Decisions

- **Nothing is stored (Q1).** Coarse Chunks, Impostors and Canopy cover are
  pure functions of the seed, the parameters and metres. No LOD cache on disk or
  between Sessions.
- **Coarse agreement (Q8, Q10).** Each coarse column is water or land, and has
  its top Material, as level 0 has at the column's centre in metres: the same
  terrain height against that column's own water level, sea or lake, the Biome
  sampled at the centre. A dry column's top is raised to at least that water
  level. This replaces the per-level half-Voxel lowering and its tie, and is
  applied at levels 1 to 5, after the downsample at 1 and 2.
  - A column under an Edit or saved change keeps the downsample's answer, so a
    drained lake or a built pier shows as it does today.
  - It decides the ground. Impostors and Canopy cover then stand on it.
  - It lives in the coarse path only. `Worldgen` at 2 m is level 0 for a World
    with 2 m Voxels, and version 3's output is frozen
    ([ADR-0033](../../docs/adr/0033-generator-version-3-is-the-only-version.md)):
    every existing worldgen test and golden stays bit-identical.
- **Tree Impostors (Q2, Q9).** At levels 1 and 2 a coarse Chunk stands every
  tree level 0 stands in its footprint, fully grown, written into the coarse
  grid so that any trunk or canopy Voxel beats air; where two Materials meet,
  the canopy wins over the trunk and either over ground. Level 1 always runs
  the draw, whatever the Server holds.
- **The placement generator (Q19).** The coarse jobs share a second level-0
  `Worldgen` with the World's seed, parameters, features and Templates, asked
  for tree, Structure and Settlement draws only, never for a Chunk. It shares
  its 1 m column cache with level 1's source, on the same lattice, and that pair
  gets its own caches, sized for level 1's ring, instead of level 0's. Level 0's
  caches are then touched by nothing coarse.
- **Level 2's trees are measured first.** Level 2's children are 2 m, so its
  draws find no warm 1 m columns; the cost per level-2 Chunk is estimated at
  several milliseconds. If ticket 03 measures it past the per-level budget,
  level 2 draws Canopy cover instead of Impostors, and the spec records it.
- **Over saved changes (Q23).** A coarse branch holding an Edit or saved change
  runs no Impostor draw: it takes the changed level-0 Voxels, coarsened with
  the same any-solid rule for tree and Template Materials. So a felled tree is
  gone at every level and a standing one keeps its silhouette.
- **Structure Impostors (Q3).** Every Structure and Settlement Plot that level 0
  stands is sampled at the coarse Voxel size, any solid beating air, at every
  coarse level. Settlement paths are ground Material.
- **Plants (Q4).** Ground cover, crops and flowers stay out of coarse levels.
- **Canopy cover (Q18).** From level 3 up a coarse column's top Voxel becomes
  the dominant species' leaf Material where the expected cover is over half:
  the Biome's tree chance times the species' canopy area times its clump noise,
  zero under water, on Structure footprints and on fields. It is raised one
  coarse Voxel above the ground where the species' mean canopy top reaches at
  least half that Voxel, so forests have a silhouette at 8 m and are a tint at
  16 and 32 m.
- **The join stays a hard cut (Q5,
  [ADR-0019](../../docs/adr/0019-a-level-is-drawn-until-another-replaces-it.md)).**
- **Shadows (Q7).** Impostors and Canopy cover cast no sun shadow: the three
  cascades end at 150 m, inside level 0's ring (160 m on Terra), and the coarse sun casts
  nothing.
- **The Quality curve (Q6, Q11, Q15,
  [ADR-0036](../../docs/adr/0036-the-quality-curve-is-a-player-preference-the-server-clamps.md)).**
  A unit `u` and a reach, so coarse level k ends at `min(u * 2^k, reach)`. The
  Client requests it; the Server clamps the unit to at least `view_distance_m`
  and the reach to the Game's `lod_distance_m`, and streams from it. Level 0
  stays the Game's `view_distance_m`. The level count, holes and overlaps are
  derived from each pair of widths by `lod_rings`' existing rules, checked for
  all four parities. Presets: **Balanced** (`u` 192 m, today's doubling) and
  **Detailed** (`u` 384 m), each with a reach from 2 to 8 km.
- **Default (Q15, Q20).** `u` 384 m and 8192 m. Terra's ceiling rises from
  3000 m to 8192 m, and the engine's default `lod_distance_m` from 4096 m.
  It becomes the default only when, on the M5 with Terra at `stationary` and
  `sprint-flight`: coarse vertex memory on the GPU, Impostors included, is at most
  192 MB; the
  whole field is drawn within 10 s of spawn; GPU frame is at most 1 ms over
  `main`; and level 0's fill time does not regress. If any fails, the default is
  the widest curve that passes, recorded here.
- **Fill order (Q21).** Coarse jobs are prioritised by distance over their
  level's Cell, so the outermost level fills first and finer ones replace it.
- **Vertical extent (Q22).** Three of a level's own Cells above and below,
  unchanged.
- **Fog (Q16).** Lighter at distance, by `reach_opacity`, chosen on captures.
- **Ground tint (Q16).** A coarse column's top colour takes the plants level 0
  would grow there, from the same density fields. Lands only if captures at the
  joins match colour better, you approve them, and coarse-Chunk time stays in
  budget; otherwise `wontfix`.
- **Settings (Q12 to Q14).** Settings gains a Distance tab after Display; the
  map and compass stay in Display. A Preset dropdown (Balanced, Detailed,
  Custom); a Reach slider (2 to 8 km) and a Detail slider (`u` 192 to 384 m)
  that move only under Custom, which remembers its own values; and a table of
  each level's Voxel size, ring, Chunk count and estimated memory, live as the
  sliders move. The table and the sliders' ends use the World's own view
  distance and ceiling. Neutral names in the UI; the mockup's notes may compare
  them with other engines. The mockup is approved.

## Testing Decisions

- **Agreement test.** Over coastal and forest windows on Terra's default seed,
  for each coarse level: water or land and top Material agree with level 0 at
  every sampled column centre (exact); a forest's edge agrees with level 0's
  tree density within one coarse column. Ticket 01 records today's rates;
  ticket 02 makes the first exact.
- **Bit-identical.** Every existing worldgen, determinism and golden test passes
  unchanged after every ticket.
- **Impostors.** A level-1 Chunk built from held children equals one built from
  generated children; a tree's Impostor stands in the coarse Voxels its level-0
  trunk and canopy occupy; a felled and evicted tree is absent at level 1; the
  placement generator's draws equal level 0's.
- **Quality curve.** `lod_rings` tests for every preset and the clamp; level 0
  is unchanged by any curve.
- **Budgets.** The packed-quads method: interleaved timing rounds and a
  separate `--instruments gpu` series (`scripts/profile-frames`), release
  `profiling` builds, quiet host, run in mirrored order against `origin/main`,
  at `stationary` (steady draw cost and memory) and `sprint-flight` (streaming
  and fill), at 01 and 09 only.
- **Shadows (ticket 07's reopening).** No GPU series: step 22 adds no caster
  and leaves level 0's radius as it is, the two triggers that ticket named. A
  test checks no coarse-level mesh is on a shadow-casting layer, so the shadows
  cost what they cost on `main`. Reopen cascade caching if either trigger
  changes.
- **Agreement compares the ground.** Water or land and top Material are read
  beneath any Impostor or Canopy cover, and only on columns no Edit or saved
  change reaches.
- **Captures (Q17).** `TESSERAE_SCREENSHOT_OFFSCREEN=1`, from fixed Eyes at 0.5,
  1, 2, 4 and 8 km on Terra, with Eyes on a Voxel plane, beside your reference
  screenshots from other engines; one `/tmp` folder, never committed, shown to
  you for each look change.
- Demo Game tests on Terra only.

## Out of Scope

- Stored or cached LODs, or reading any other engine's data.
- A dither band at the join.
- Sun shadows from coarse levels, or a fourth cascade.
- Plants other than trees at coarse levels, beyond the tint.
- Several Eyes on one Server.
- Raising level 0's `view_distance_m`.
- Baked cards or billboards for trees; reopen only if captures show coarse
  trees as blobs.
- Any change to Generator version 3's output.

## Further Notes

- Prior-art findings, in our own words: other engines with distant terrain
  build their far field from chunks already generated, store it per level, and
  get trees only from that generation; none has Impostors. Both their default
  curves hold a level to about `386 * 2^k` m, about 2 pixels per coarse Voxel at
  1080p. One also thins terrain vertically, with a per-column slice cap that
  turns overhangs into walls, which DESIGN.md rejects.
- Measured cost of exact tree draws on level 0's lattice: 0.1 to 0.4 ms per
  level-1 Chunk with the column cache warm; 1 to 45 ms per level-3 Chunk,
  against 3 to 4 µs today. That is why trees are Impostors only at levels 1
  and 2.
- Terra's tree density ranges from about 1,800 trees a km² (dry) to 41,000
  (jungle), 7 to 169 per level-1 footprint.
- DESIGN.md's LOD section and step 22 note are updated when the branch lands.
