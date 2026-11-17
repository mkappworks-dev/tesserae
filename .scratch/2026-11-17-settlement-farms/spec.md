Status: ready-for-agent

# Settlement farms

Farms, orchards and paddocks round every Settlement, and vegetable gardens among
its houses. Builds on step 20 (Settlements,
[ADR-0029](../../docs/adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md))
and step 18 (Plants), and lands with step 17 on its branch. The Engine lays the
ground out; a Game declares what stands there.

## Problem Statement

A Settlement is houses on roads in a ring of forest or grass. Nothing feeds it:
its crop fields, which Biomes scatter over open land, are kept off its disc and
its approach paths, so the nearest field is wherever chance put one. A village
has farmhouses and barns and no farm.

## Solution

- **Outskirts.** A Settlement declares outskirt Plots: Buildings laid beside its
  approach paths just past its disc, within its margin, by the same Plot rules
  as the disc's (weight, minimum, maximum). A field, an orchard or a paddock is
  a Template like any other Building.
- **Gardens.** A vegetable garden is a Building the disc's Plot rules list like
  a cottage.
- **Terra declares them.** Field Templates (wheat, carrots, potatoes, mixed) of
  farmland and crop rows with a lane, an orchard Template (apple, cherry or
  orange trees in rows), a fenced paddock with a gate, and a fenced vegetable
  garden. A town has the most outskirts, a village several, a hamlet one or two.

## User Stories

1. As a Player arriving at a village, I walk its approach road between fields
   and orchards before I reach the houses.
2. As a Player in a village, I find gardens between the houses.
3. As a Player, the crops in a Settlement's fields ripen and can be harvested
   like any other planted crop.
4. As a Game author, I list outskirt Buildings per Settlement like its Plots,
   with weights and counts.
5. As a Player with a World made before this, my Settlements are unchanged.
6. As a Player at 0.5 m Voxels, the same fields are in the same place in
   metres.

## Implementation Decisions

- **Outskirt Plots** are tried at slots along each approach path past the disc,
  both sides, set back from the path as disc Plots are. A Plot stands only if
  its footprint and apron lie outside the disc and inside the disc plus margin,
  clear of every other Plot and road, on ground the existing Plot rules level.
  Their order is drawn from the Settlement's hash after the disc's Plots, so
  adding outskirts never moves a disc Plot.
- **Claims.** Outskirt Plots are part of the laid Settlement: they stand with
  it, and their footprints and aprons join the footprints that fields, trees
  and scatter keep off. Single Structures already keep off the margin.
- **Contract.** `settlement-def` gains an optional `outskirts` list of the same
  plot records and an optional `outskirts` count range, appended to the
  unreleased 0.3 Declaration (`docs/contract-compatibility.md`); the TOML loader
  and the save's Settlement snapshot follow, defaulted when absent so a saved
  World lays out as before.
- **Crops in Templates** are a crop's first Stage on farmland, so they grow and
  are harvested by the existing Plant rules; trees in an orchard are a grown
  tree's Voxels, so they fell and drop like a wild one.
- **Templates** come from the mockup's builders (`mockup/structures.js`, exported
  by `node mockup/export-templates.mjs`), never edited by hand in Terra.

## Testing Decisions

- Version 1 and 2 digests (`tests/settlement_worldgen.rs`,
  `settlement_snapshot.rs`, `generator_version.rs`) pass unchanged.
- A Settlement declaring outskirts stands each one whole, outside its disc and
  inside its margin, in any Chunk order and the same in metres at 1 and 0.5 m.
- No field, tree or scatter Plant stands on an outskirt Plot or its apron.
- A Settlement without outskirts lays out exactly as before.
- A snapshot without outskirts restores none.
- Captures, offscreen: a village from its approach road and from above, kept in
  one `/tmp` folder and deleted at review.

## Out of Scope

- Animals in the paddocks (step 25).
- Fields that follow the ground's contours or irrigation from rivers.
- Farms round Settlements of the other demo Games (their own content later).
