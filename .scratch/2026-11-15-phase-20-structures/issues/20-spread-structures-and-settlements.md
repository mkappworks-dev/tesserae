# 20: Spread Structures and Settlements evenly but irregularly

Status: done

Blocked by: 03, 14, 15

Spec: [Phase 20](../spec.md).

## What to build

Structures and Settlements are spread across a World, and buildings across a
Settlement, by a blue-noise scheme drawn from the World seed: never in rows or
on a visible grid, never clumped, and different in every World.

Today a Settlement's centre is jittered only by the slack between its lattice
spacing and twice its radius plus margin. At the smallest allowed spacing that
slack is zero, so centres stand on an exact grid, and every listed cell holds
one. Inside a Settlement, Plots are packed every 2 m along each road, nearest
the centre first, so every village is a tight cluster.

- **Across the World**, for single Structures and Settlement centres alike:
  - Each lattice cell draws a candidate anywhere in the cell from the seed.
  - A candidate stands only if no candidate in a neighbouring cell with a
    higher draw lies within the minimum distance. The minimum distance is the
    declared spacing for Structures, and twice the reach (radius plus margin)
    for Settlements.
  - This is a pure-function Poisson-disc scheme: each cell asks only its
    neighbours' candidates, never the ground rules, so it stays cheap and
    independent of Chunk order.
  - A Biome's Settlement row gains a `chance`, as a Structure row already has,
    so not every cell holds one.
- **Inside a Settlement**, Plots are spread over the whole disc rather than
  packed from the centre out:
  - Slots along the roads are tried in a seeded order with seeded gaps, so
    there is open ground between houses.
  - Each Plot sits a seeded distance back from the road, within a range.
  - The density falls off towards the edge, so outer buildings stand further
    apart.
  - Roads still branch from the centre, and doors still face their road.
- **Roads and paths**: a Settlement's roads are laid in its declared path
  Material, so a Game picks stone or dirt (for example cobble in a town, packed
  dirt in a hamlet). They narrow and wear towards the disc's edge: seeded worn
  patches let the Biome's surface show through. Two or three of the roads run
  on past the disc as approach paths, a seeded 12 to 40 m and never beyond the
  Settlement's margin, so they stay inside the ground it already claims. They
  follow the same grade rules and fade out into the terrain rather than
  stopping square.
- Everything is computed in metres from the seed and the cell, and is
  identical at every Voxel size.

## Acceptance criteria

- [x] No two Settlements' discs plus margins overlap, and no two Structures
      of one row stand closer than their spacing, across a large scan in any
      Chunk order.
- [x] Settlement centres in a large scan show no grid: the spread of their
      positions within their lattice cells is close to uniform, tested.
- [x] Two seeds give different layouts; one seed gives the same layout in
      any Chunk order and at 1.0, 0.5 and 0.25 m.
- [x] Within a Settlement, the gaps between neighbouring Plots vary, and
      Plots reach the outer half of the disc.
- [x] A Settlement's roads are its path Material, wear towards the edge, and
      at least two run past the disc as approach paths that end inside the
      margin and fade out, tested.
- [x] WIT, TOML, validator and snapshot carry the Settlement row's `chance`,
      and the WIT copies are synced.
- [x] The benchmark (ticket 18) still meets its margin.

## Comments

Across the World (`structure_draw` / `structure_candidate` in
`worldgen/structures.rs`, `site_draw` / `site` in `worldgen/settlements.rs`):

- Each cell's candidate, its chance and its priority come from hashes alone.
  A Structure's footprint centre is drawn uniformly in the cell inset by
  0.5 m, so its corner on whole metres still leaves the centre in the cell; a
  Settlement's centre is any whole metre in the cell.
- It stands if no neighbour's candidate with a higher (priority, cell) is
  within the minimum distance: the row's spacing for Structures, twice the
  reach (disc plus margin) for Settlements. The validators' floors (spacing at
  least twice the longer side, or twice the reach) make that distance at most
  a cell, so only the 8 neighbours are asked: 9 hash draws a cell, no ground.
  Both are memoised per cell by ticket 18's `Layouts`, so the cost is paid
  once per generator; every cached value still depends only on the generator
  and the cell.
- ATTEMPTS is dropped. A retry elsewhere in the cell could stand nearer a
  neighbour than the spacing, and retries inside a shrunken disc would need the
  check widened to 5 by 5 cells and lower the density for every row. A
  candidate the ground refuses leaves its cell empty; a Game wanting more on
  rough or wet ground lowers `spacing-m`.
- Settlements still claim before single Structures; a Site whose layout falls
  short of its minimum still keeps single Structures out of its reach (as
  before).
- Contract: `biome-settlement` gained `chance` (above 0, at most 1). TOML
  defaults it to 1.0 when absent. Core `BiomeSettlement.chance`,
  `SavedBiomeSettlement.chance`, `Settlement::reach_m()`. Both WIT copies
  synced; docs/contract-compatibility.md and ADR-0029 amended.

Inside a Settlement (`lay_out`, `slots`, `roads`, `rasterise`):

- Slots: on each side of each road at seeded gaps of 1 to 4 m, tried in a
  seeded order (sorted by a per-slot hash); each Plot stands back a seeded 0 to
  2 m past its apron. A Plot keeps an extra clearance from other Plots of
  4 m times its distance from the centre over the radius, so density falls
  off outwards. Minimum rules still come first in the queue.
- Roads narrow from 1.5 m half width at the centre to 1.0 m at the disc's
  edge. Each metre cell beyond half the radius wears (shows the Biome's
  surface, still graded) by its own seeded draw, up to 25% at the edge.
- 2 or 3 of the 3 or 4 roads run on past the disc as approach paths, a seeded
  12 to 40 m, cut to the margin less their 0.9 m half width, heading only
  outwards and keeping the grade rules. Wear rises with the square of the
  distance along the path to 100% at its end, so it fades out. No cell is laid
  past the margin, so no claim or bounds change. The road grid now covers the
  disc plus `min(margin, 40.9 m)`.
- Trees, scatter and fields keep off the approach paths: each step point past
  the disc adds a small disc to the Town's footprints.

Tests changed (positions no longer on the lattice; no digest changed):

- settlement_worldgen: Settlement centres are found from each well rather
  than assumed at lattice-cell centres; the scan skips marks the scan edge
  cuts; the whole-across-borders test scans 96 m (was 64) and asks for at
  least one crossing (was 3); the door test scans 128 m and looks 10 cells
  along three lines (setback and diagonal roads put the road further, a worn
  cell is no path); the wilderness test allows single Structures removed by
  a Site that never laid out, and checks two Chunks in.
- structure_worldgen: the lake and foundation tests use 10 m spacing (and the
  lake 128 m) since a refused draw is no longer retried.
- New: spacing and no-grid scan for Structures, seeds differ (both suites);
  no Settlements meet and their in-cell offsets are even over a 384 m scan;
  Plots reach the outer half with uneven gaps; roads wear towards the edge and
  two or more approach paths end inside the margin and thin out; the loader
  carries `chance`, defaults it and refuses 0 and 1.5; the snapshot keeps it.

Benchmark (`cargo bench -p tesserae-core --bench structures`, load 11 to 18;
us a Chunk, warm / cold, bare in brackets; each variant centres on what it
stands nearest the origin, which moved, so bare differs by variant):

| Variant           | 1.0 m           | 0.5 m          | 0.25 m         |
| ----------------- | --------------- | -------------- | -------------- |
| house, 64 m       | 207 / 232 (206) | 104 / 127 (89) | 99 / 123 (66)  |
| house, 14 m       | 264 / 300 (245) | 115 / 143 (83) | 100 / 123 (67) |
| hall at the cap   | 227 / 319 (214) | 182 / 248 (95) | 189 / 254 (69) |
| village, 12 Plots | 237 / 299 (218) | 129 / 188 (73) | 112 / 172 (61) |
| town at the cap   | 257 / 345 (239) | 204 / 291 (151)| 178 / 265 (109)|

Worst: the hall cold at 0.25 m, 0.185 ms over bare, inside the 0.25 ms
margin. Sky and deep-rock Chunks keep their early-outs.

Games (taken over from ticket 17):

- Each Game's village row sets `chance`, thinner where its lattice is
  denser: Tidewater 0.6 (104 m), Terra 0.7 (128 m), Ember 0.8 (192 m),
  Meadow 0.9 (256 m), Lumen 1.0 (320 m); Terra's town 1.0 (384 m). The
  starter declares no Settlement rows. The per-Game test still finds a
  Settlement within 320 m of spawn at seed 7 in every Game; HALF_M is
  unchanged.
- Terra's town is proved: `terra_lays_out_a_town_of_cobbled_roads` scans
  128 m round (-416, -416) at seed 7 and finds a well with 428 cobbled road
  cells (outside every building) within 60 m of it; only the town lays
  cobblestone roads, and no wild Structure stands in its reach.
- Approach paths are cut to the margin less their half width, and no cell is
  laid past the margin, so the Games' 24 m margins hold them (at most
  23.1 m).
- TOML rows take `chance` and default it to 1.0; a WASM Declaration must
  give it.
