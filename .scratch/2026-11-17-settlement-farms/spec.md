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
- **Fuller farms, not everywhere.** After the first play-test: a Settlement
  farms by chance (most towns, many villages, some hamlets), and one that farms
  stands at least four fields or none. Fields are larger, stand two deep from
  the path, and some wheat fields are already ripe.

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
7. As a Player, some Settlements are ringed by large farms, some have none, and
   one never has a single lonely field.
8. As a Player, I sometimes arrive at harvest time: a wheat field already ripe.

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
- **Crops in Templates** may be any of a crop's Stages on farmland, so they
  grow and are harvested by the existing Plant rules; a later Stage is a field
  planted earlier. It is Obtainable when its crop's first Stage is. Trunk,
  canopy and fruit stay refused, since Felling tells a grown tree from a build
  by them; an orchard holds saplings.
- **Chance and minimum.** `outskirts` gains a `chance` that the Settlement
  farms at all, drawn from its hash, default 1. The count range's minimum is
  all-or-nothing: when fewer fit, the Settlement lays no outskirts.
- **Second row.** A slot is tried against the path first, then once more set
  back behind a first-row Building's depth, so fields stand two deep.
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
- Over many Settlements, about `chance` of them farm, and each that does stands
  at least the minimum.
- A Template naming a later crop Stage loads, and one naming a tree's trunk
  still fails.
- Captures, offscreen: a village from its approach road and from above, kept in
  one `/tmp` folder and deleted at review.

## Out of Scope

- Animals in the paddocks (step 25).
- Fields that follow the ground's contours or irrigation from rivers.
- Farms round Settlements of the other demo Games (their own content later).
