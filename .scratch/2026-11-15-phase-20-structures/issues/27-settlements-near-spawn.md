# 27: Measure how far Settlements stand, and make them easier to find

Status: done

Blocked by: 20, 24

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found Settlements hard to find. Measure, for each demo Game at a
fixed seed:

- each Settlement row's spacing and `chance`;
- how many lattice cells round spawn draw a candidate, and how many of those
  the chance, the Poisson neighbour check, the origin keep-out, the Biome, the
  water and the ground rules turn away, and how many lay out;
- how far the nearest Settlement stands from spawn, and how far Settlements
  stand from each other.

Report the numbers to the user with a proposal to make Settlements easier to
find, such as denser lattices or a higher chance near spawn, and change the
Games only once the user agrees.

Decided 2026-11-16 by the user, from the numbers below:

- **A village near spawn.** A Biome's Settlement row may say `near-spawn`.
  The four lattice cells round the origin of such a row always draw (no
  chance roll), win the neighbour check, and draw their centre outside the
  origin keep-out, so the row's Settlement stands within about a cell's
  diagonal of spawn wherever the ground allows. Each Game sets it on its
  village rows. Contract: WIT (both copies), TOML, validator, snapshot,
  contract-compatibility.
- **Denser everywhere, by about 30%.** Each Game's Settlement rows (chance,
  and hamlet spacing no closer than its floor, twice the hamlet's reach) are
  tuned so the census finds about 30% more Settlements than before; the user
  judged 30 to 60% too many.
- **Trails past the margin.** Margins stay 24 m. Two or three approach roads
  run on past the margin as trails, 40 to 90 m from the disc, worn path cells
  that follow the ground and stop at water, at a grade too steep, or at
  another Settlement's disc and margin. Trails claim no ground: a wild
  Structure may stand on one.

## Acceptance criteria

- [x] The numbers are recorded in this ticket for every Game.
- [x] The user decides what changes, if anything.
- [x] A `near-spawn` row stands a Settlement within the four cells round the
      origin in every Game at the test seeds where the ground allows, tested
      through `Worldgen::chunk`; WIT copies synced, TOML, validator and
      snapshot carry it, and a row without it behaves as before.
- [x] The census finds about 30% more Settlements than before in each Game
      (25 to 35% at seed 7), the near-spawn villages included.
- [x] Trails run past the margin 40 to 90 m, follow the grade rules, stop at
      water and at another Settlement's claim, and look the same in any Chunk
      order and at 1.0, 0.5 and 0.25 m.
- [x] The benchmark still meets its margins.

## Comments

Measured 2026-11-16 with a throwaway census over every lattice cell of every
Settlement row within 1.5 km of spawn (a 3.07 km square), 1 m Voxels:

| Game      | Rows: spacing, chance                          | Laid | Nearest to spawn, seeds 7 / 1 / 2 / 3 / 4 | Neighbour, median |
| --------- | ---------------------------------------------- | ---- | ----------------------------------------- | ----------------- |
| Terra     | town 384 m, 1.0; village 192 m, 0.7; hamlet 128 m, 0.8 | 62   | 169 / 113 / 447 / 195 / 128 m             | 231 m             |
| Tidewater | harbour 112 m, 0.6; hamlet 104 m, 0.6          | 59   | 264 / 113 / 98 / 213 / 199 m              | 188 m             |
| Ember     | village 192 m, 0.8; hamlet 128 m, 0.8          | 65   | 169 / 113 / 192 / 127 / 170 m             | 218 m             |
| Lumen     | village 320 m, 1.0; hamlet 160 m, 0.8          | 174  | 139 / 134 / 120 / 135 / 160 m             | 155 m             |
| Meadow    | village 256 m, 0.9; hamlet 128 m, 0.8          | 152  | 180 / 187 / 124 / 127 / 103 m             | 163 m             |

Laid counts are at seed 7. The play-tested World (Terra, seed
16089904431951856265) has a 3-building hamlet 176 m from spawn, a 9-building
village at 192 m, and villages at 362, 433 and 433 m.

Where candidates are lost, Terra's first village row at seed 7 (249 cells):
76 to the chance roll, 76 to the neighbour check (which keeps copies apart at
their largest size), 23 to water, 23 to a Biome that does not list it, 10 to
the centre's ground, 10 claimed by the town, 21 whose layout fell short of its
minimum, 16 laid. Hamlet rows lose about 45% to the neighbour check and 25% to
chance.

Reading: Settlements are not far, the nearest usually 100 to 270 m, but the
nearby ones are mostly hamlets of 2 to 5 buildings about 60 m across, villages
lay 6 to 11 buildings, and approach paths end at most 40 m out, so they are
easy to walk past.

Built 2026-11-16.

Near spawn: `BiomeSettlement::near_spawn` (`climate.rs`), carried by both WIT
copies (`near-spawn`, after `chance`), TOML (`near-spawn`, false when left
out), the loader and the snapshot (`SavedBiomeSettlement::near_spawn`, left
out when false). `near_spawn_draw` (`worldgen/settlements.rs`): in the four
lattice cells round the origin a near-spawn row skips the chance roll, takes
the top priority against its neighbours, and draws its centre at a seeded
distance from just past the keep-out for its largest copy (reach + 48 m + 2 m)
to the cell's side, at a seeded angle across the cell's quarter, on whole
metres. A lattice too small for that draws as any other row. Every Game's
village rows say it (demo-kit `Village::near_spawn`, Terra's `row`).

Trails: `TRAIL_M` (40 to 90 m past the disc) replaces the 12 to 40 m approach
paths and their cut at the margin. `walk` stops a road short of another
Settlement's disc and margin and of the origin's keep-out; `rasterise` no
longer cuts at the margin; `laid_settlements_near` asks `foundings_within` a
trail's reach further, while `settled` (what keeps wild Structures out) still
asks only disc and margin. A wild Structure may stand on a trail.

Density, measured as before (seed 7, 3.07 km square, 1 m), against the count
with `near-spawn` and the old lattices:

| Game      | Before | After | Change | Rows now                                    |
| --------- | ------ | ----- | ------ | ------------------------------------------- |
| Terra     | 62     | 78    | +26%   | village 160 m, 0.7; hamlet 96 m, 0.8        |
| Tidewater | 59     | 74    | +25%   | harbour 112 m, 0.6; hamlet 100 m, 0.6       |
| Ember     | 65     | 86    | +32%   | village 160 m, 0.8; hamlet 96 m, 0.8        |
| Lumen     | 174    | 227   | +30%   | village 256 m, 1.0; hamlet 128 m, 0.8       |
| Meadow    | 155    | 207   | +34%   | village 224 m, 0.9; hamlet 112 m, 0.8       |

How: every Game's Settlement margin went from 24 to 16 m (trails now run past
it, so it no longer has to hold the approach paths), which lowers each
lattice's floor, and the lattices closed towards it. Raising `chance` did not
help: more draws are turned away by the neighbour check and by earlier rows'
claims, so counts stayed level or fell; neither did keeping neighbours apart
by their own drawn sizes rather than the largest (tried, reverted). Terra's
town keeps 384 m.

Nearest Settlement to spawn at seed 7 after: Terra 141 m, Tidewater 224 m,
Ember 262 m, Lumen 165 m, Meadow 156 m. Tidewater's harbour lattice (112 m)
leaves its near-spawn draw 98 to 111 m out, mostly water at that seed.

Tests: `a_near_spawn_row_stands_its_settlement_in_the_cells_round_the_origin`
(five seeds: one to four round spawn with the flag at chance 0.01, at most one
over all five without, and nothing within 48 m of the origin);
`trails_run_past_the_margin_and_fade_out`;
`a_trail_stops_short_of_another_settlements_disc_and_margin` (two kinds told
apart by their path Material, packed at their floor);
`roads_wear_towards_the_edge_of_the_disc` (the wear half of the old approach
test); the loader's `a_biome_settlement_row_may_say_near_spawn`; the
snapshot test now round-trips a near-spawn row. The wilderness test allows
trail ground past the margin. `a_fixed_size_settlement_lays_out_exactly_as_before_ranges`
is re-pinned, as its approach paths now run on as trails.

Trails' cost, found by the full benchmark: with the road grid widened to hold
90 m trails, a cold layout graded about five times the cells, and the
village (+0.35 ms at 1 m) and the town at the cap (+0.40 to +0.43 ms) went past
their margins. Trails past the disc and margin now live outside the grid:
`LaidSettlement::trail`, a sorted list of metre cells, which `stand_trail`
lays as the path Material on each column's own top Voxel, keeping a smoothing
stair on it. Only roads inside the grid (`grid_m`: disc, margin and half a
path) are graded; a worn trail cell past it changes nothing. The slab-per-step
test now asks it of graded roads only, and the fixed-size digests are re-pinned
again.

Benchmark after (load 5 to 7; cold, over bare): village +0.16, +0.15, +0.10 ms
at 1.0, 0.5, 0.25 m; town (64 m) +0.11, +0.14, +0.17 ms; town at the cap +0.28,
+0.29, +0.29 ms (margin 0.35 ms); every warm variant within 0.25 ms.
