# 14: `flora` and Terra content

Status: ready-for-agent

Blocked by: 05, 06, 11, 12, 13

Spec: [Phase 18](../spec.md).

## What to build

Plants in the Mods. `flora` declares grass as a `spreader`, kelp as a `column`,
an oak `tree` over its wood and leaves, and flowers as `spreader`s, with their
`cross` Shapes and textures. Terra declares wheat (`crop`), a berry bush (`crop`
with `harvest-to`), sugar cane (`column`), mushrooms (`spreader`) and vines
(`climber`), with Seeds, produce and food Recipes; Terra's Biomes place them as
surface features. Terra's standalone Wood surface feature is removed; Wood is
taken from grown oaks, which its temperate Biome places. Art is drawn by
`scripts/generate-mod-art`.

## Acceptance criteria

- [ ] A new Terra Survival World shows meadows, forests and kelp beds; planting
      wheat, waiting and harvesting yields food; a berry bush fruits again.
- [x] Terra's apple tree bears apples that are broken to collect and restore hunger;
      the standing tree grows new fruit elsewhere.
- [x] No Wood Voxel is generated on its own; a new Survival World still reaches
      Wood, a Workbench, a Chest and a Door by felling and breaking oaks.
- [ ] `meadow`, `ember` and `tidewater` load with `flora`'s new rows and their
      starter tests pass.
- [x] `tesserae validate` passes for `flora` and every Game.
- [x] No art is hand-drawn; the generator script is the source.

## Comments

Reviewed: boxes 2, 3, 5 and 6 hold (`demo_games` tests, and `tesserae validate` run on `flora` and the five Games with `--mods mods`). Box 1 needs a played World. Box 4 names the Games' own starter tests, which were not run; only the `demo_games` loads were.

Left `ready-for-agent`: no GUI run, and the first two boxes are met only by data
and generation tests, not by a World played through.

- `flora` now `depends = ["strata"]` (its Plants name strata soils), so the
  load order of `ember`, `meadow` and `tidewater` puts `flora` after `strata`.
  Their tests and the texture pins moved with it.
- `flora` declares `cross`, grown `oak_log`/`oak_leaves`, `oak_sapling`, the oak
  `tree`, `tuft`/`poppy`/`daisy` `spreader`s and a `frond` Material. Kelp's
  `column` is declared by `tidewater` (it names `waters:water` in `stands-in`,
  which `flora` cannot) with seed `flora:frond`; `meadow` places the oak, tuft
  and poppy; `tidewater`'s lagoon places kelp.
- Terra declares its own `cross`/`plane` Shapes and Plants (oak, apple, wheat,
  berry, cane, kelp, mushroom, vine), bread and berry pie, and drops
  `terra:wood` from the temperate Biome. A Biome holds at most four features, so
  they are shared: temperate oak, apple, wheat; dry cane, berry, kelp; cold
  mushroom, kelp. A feature cannot say "near water", so cane stands anywhere in
  the dry Biome and only grows by water. Vines are found by planting only.
- Apple fruit Materials are cutout cubes drawn as a small apple on clear
  ground. The maintainer asked for a registered box Shape, but a tree's Voxels,
  fruit included, are always placed as cubes by growth and generation
  (`Voxel::cube` in `plant/tree.rs`), so a Shape needs a `fruit` field in the
  contract first.
- A Seed places with the Shape the Player selected: nothing gives a Seed a
  default `cross`. Ticket 13 should make a Seed of a plant Material place its
  flat Shape, or the contract needs a field for it.
- `scripts/build-guests` stops at the `catalog` test guest (`ShapeDef` has no
  field `form`), so the `server::mods` tests that read `target/guests` fail
  here; Terra and the demo Games were built by its last block alone.
