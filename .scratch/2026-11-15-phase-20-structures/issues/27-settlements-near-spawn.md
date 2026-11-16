# 27: Measure how far Settlements stand, and make them easier to find

Status: ready-for-agent

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
- **Denser everywhere.** Every Game's Settlement rows draw at chance 1.0, and
  hamlet lattices close to their floor, twice the hamlet's reach.
- **Trails past the margin.** Margins stay 24 m. Two or three approach roads
  run on past the margin as trails, 40 to 90 m from the disc, worn path cells
  that follow the ground and stop at water, at a grade too steep, or at
  another Settlement's disc and margin. Trails claim no ground: a wild
  Structure may stand on one.

## Acceptance criteria

- [x] The numbers are recorded in this ticket for every Game.
- [x] The user decides what changes, if anything.
- [ ] A `near-spawn` row stands a Settlement within the four cells round the
      origin in every Game at the test seeds where the ground allows, tested
      through `Worldgen::chunk`; WIT copies synced, TOML, validator and
      snapshot carry it, and a row without it behaves as before.
- [ ] Every Game's Settlement rows draw at 1.0 and its hamlet lattice is at
      its floor; the census finds more Settlements than before in each Game.
- [ ] Trails run past the margin 40 to 90 m, follow the grade rules, stop at
      water and at another Settlement's claim, and look the same in any Chunk
      order and at 1.0, 0.5 and 0.25 m.
- [ ] The benchmark still meets its margins.

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
