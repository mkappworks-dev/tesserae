# 03: `timber`, `brickwork` and `glassworks` carry Items and Recipes

Status: done

Blocked by: 02

Spec: [Shared building Materials](../spec.md).

## What to build

The three shared Mods gain the Materials, Items, Recipes, voxel-uses, Shapes and
`depends` in the spec's tables, so each can be built from shared world Materials
at `workshop`'s Stations.

- `timber` (depends `flora`, `crops`, `workshop`): `planks` (kept),
  `pine_planks`, `dark_planks`, `red_planks`, `frame`, `dark_frame`, `shingles`,
  `thatch` (kept), `awning`, and the `door` Door, 2 m wide and 3 m tall, and
  `gate` Door, 1 m by 1 m, with their open states. A new `leaf` Shape, 3/16
  thick against its own +Z, is fixed by both Doors; the centred `panel` and
  `beam` stay.
- `brickwork` (depends `strata`, `workshop`): `cobble`, `brick` and `tile`
  (kept), `ashlar`, `slate`, `red_tile`, `whitewash`.
- `glassworks` (depends `strata`, `flora`, `workshop`): Recipes for the existing `glass`
  and `lantern`. `frosted_glass` stays without one.

New Materials take Terra's colour for the Material they replace (the spec's
mapping table), as a flat appearance until the art ticket lands. The colour
session's art and appearance for the kept Materials are not touched. Each README
lists its Recipes and "Used by".

## Acceptance criteria

- [ ] Each Mod composes with only its `depends` and passes validate.
- [ ] Under ticket 02's fixture Game, every Material each declares is Obtainable
      through Stations that are.
- [ ] A `timber` door opens and closes as Terra's does: 2 m by 3 m, hinged on
      the aimed half, one panel against its jamb when open.
- [ ] `bog` still composes, and its reaction still makes `brickwork:brick`.
