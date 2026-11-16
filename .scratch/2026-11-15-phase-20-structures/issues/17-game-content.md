# 17: Every Game ships Templates and a Settlement

Status: done

Blocked by: 04, 05, 06, 07, 12, 13, 15, 16

Spec: [Phase 20](../spec.md).

## What to build

Each of Terra, Tidewater, Ember, Lumen and Meadow declares fifteen Templates
from Materials Obtainable in it, as the approved mockup shows: ten Settlement
buildings and five wilderness Structures. Each building kind has its own shape:
a steep-roofed cottage, a jettied half-timbered family house, an L-shaped
farmhouse with a porch and cellar, an inn with a hipped roof and cellar, a long
timber-framed warehouse of chests, a lean-to blacksmith with a forge chimney, a
brick bakery with its oven at the back, a market stall with an awning, a barn
with a two-pitch roof, and a well. Each kind uses its own Materials from ticket
16, so no two buildings in a Game share walls and roof. Terra's stone bridge
humps over a brick-ringed arch, Meadow's footbridge stands on log trestles, and
Tidewater's rope bridge sags between two raised landings. Each declares at least
one Settlement that uses them, and Terra also a town. Stations a building needs
(an anvil, a forge, an oven) are declared with the step 16 machinery. Each
Game's Biomes list them. The SDK starter declares one Template as the documented
example. The Engine ships none.

## Acceptance criteria

- [x] A World of each Game shows its Templates and a Settlement, level, whole
      and furnished, with cellars underground.
- [x] Each Template matches its approved mockup in shape and Materials, by a
      test that compares the declared cells to the mockup's export or by review
      recorded under Comments.
- [x] Every Game passes the Obtainable check.
- [x] The starter's Template loads in its test.
- [x] A World with no Game has no Template.
- [x] Existing trees are unchanged.

## Comments

- Built on 2026-11-16. Every Game declares fifteen Templates, its own door,
  chest, workbench, glass and lamp (ticket 16's open question), the `panel` and
  `table` Shapes they stand in, the `place` Tool, and Settlements its Biomes
  list beside its wilderness Templates.
- One source of truth: the mockup's builders. `node mockup/export-templates.mjs`
  loads `structures.js` and `structure-materials.js` and writes each Game's
  cells to `mockup/templates/<game>.json` (prettier-ignored) and to
  `games/<game>/src/templates.rs`, generated Rust that declares them, so the
  guests stay WIT-only and Terra passes `check-terra-boundary` (it allows no
  `include_str!`). `--check` runs in CI's Lint job. The test
  `every_games_templates_are_the_cells_the_approved_mockup_exports` loads each
  Game through the Mod loader and compares every cell, size, Ground layer,
  foundation and tolerance with the JSON: criterion 2 is met by test, with no
  exception.
- Export rules: a mockup cell's turn takes +Z to -X, an orientation byte's to
  +X, so orientation = (4 - turn) % 4. Shapes `cube`, `slab`, `stair`, `post`
  are `core:`; `panel` and `table` are the Game's own. The Ground layer is the
  mockup's y = 0 (so cellars, wells and abutments are underground), the
  foundation is the style's `base`, and wilderness Templates take a 2 m level
  tolerance (buildings keep 1 m; Plots are levelled anyway).
- Mockup changes, beyond the furnishings: the bakery's oven cell (was a lamp)
  is the Game's oven Station and one smithy hearth cell is its forge, so the
  buildings hold the Stations the ticket names. FIXED now names each Game's own
  furnishings; the unused EXPORTED list is gone.
- Settlements: a village (Tidewater's `harbour`, Ember's `hamlet`) of all ten
  building kinds round the well, 12 Plots (8 in Tidewater), minimum 3, margin
  24 m, roads in packed earth (`terra:dirt`, `strata:soil`, laid by each
  Game's ground). Terra also has a `town`: radius 60 m, 22 Plots, minimum 8,
  cobblestone roads, 384 m spacing, listed before the village in `temperate`.
  Village radius 40 m (Tidewater 28 m, its islands are small); spacing per
  Game for how often the ground takes one: Terra 128, Tidewater 104, Ember 192,
  Meadow 256, Lumen 320 m (flat Lumen and Meadow take nearly every cell).
  Wilderness rows: 256 m, chance 0.5, each Template in one or two Biomes.
- Doors: the cottage, family house, farmhouse, inn and bakery stand on their
  closed Door, found through the Game's Door voxel-use. The smithy, warehouse
  and barn are open-fronted and the well and stall have no front: with no
  door their +Z side faces the road, which is where the mockup opens them.
- Survival does not get the place Tool for free: like every Tool it is
  declared by the Game, and Use in Survival is refused `CreativeOnly`
  (ticket 10), unchanged.
- The Obtainable check refused nothing in the five Games and is not narrowed.
  The starter's door had no Recipe, so its hut would have been refused: the
  starter gained a door Recipe at its workbench (content fixed, not the check).
- Existing trees are unchanged: trees stay parametric Plants and no worldgen
  digest moved. `the_demo_games_textures_are_unchanged` was re-pinned for
  Tidewater, Ember, Lumen and Meadow, whose furnishings add textures; before
  re-pinning, the fingerprint of every pre-existing texture was checked equal
  to its old pin, and `export-blocks` rewrote none of the existing PNGs. Old
  Worlds keep their digests: their snapshot has no Templates.
- Tests (`server/mods/demo_games/structures.rs`): the export match; each
  Settlement uses all ten buildings, none wider than its radius, doors as
  above; every Game holds the place Tool; a World with no Game has no
  Template; and, through `Worldgen::chunk` at seed 7 in 320 m of the origin,
  each Game stands Settlement buildings whole (every Voxel and Air cell),
  level (nothing under its lowest layer is air), furnished (door, chest,
  workbench, lamp, glass) and a wilderness Structure, with every cellar's rim
  in the ground. It asserts presence, not positions. Terra's town is not in
  that square; nothing asserts one stands.
- `export-blocks` now also drops a Game's Settlements (their roads name shared
  Mods' Materials) and exports the furnishings' textures for the mockup.
- For ticket 20: when it adds `chance` to a Biome's Settlement row, set it in
  each Game's `structures.rs`; if its spread moves Settlements, the scan's
  `HALF_M` may need widening. No dev capture was taken here.
