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
- [ ] Terra's apple tree bears apples that Use picks and that restore hunger;
      picked fruit grows back while the tree stands.
- [ ] No Wood Voxel is generated on its own; a new Survival World still reaches
      Wood, a Workbench, a Chest and a Door by felling and breaking oaks.
- [ ] `meadow`, `ember` and `tidewater` load with `flora`'s new rows and their
      starter tests pass.
- [ ] `tesserae validate` passes for `flora` and every Game.
- [ ] No art is hand-drawn; the generator script is the source.
