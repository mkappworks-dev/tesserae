# 17: Plants carry their Shape

Status: done

Blocked by: 12, 14

Spec: [Phase 18](../spec.md), amendment 2026-11-14.

## What to build

A Plant's Voxels have a Shape the Plant declares, not the Player's choice and not
a cube. Found while building the content: a seed places with whatever Shape the
Player selected, so planted wheat is a cube unless they pick `cross`, and growth
and worldgen always place tree Voxels with `Voxel::cube`, so the approved 3D
apple cannot be expressed in data.

- `plant-def` gains `shape: option<string>`: the Shape every single-Voxel Plant
  (crop Stages, column, spreader, climber, sapling) is placed, grown and
  generated with. Placing its seed ignores the Player's Shape and Orientation
  choice, as a Door's fixed Shape does ([ADR-0026](../../../docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md)),
  and the Catalogue and Client placement ghost report the fixed Shape. A climber
  keeps its Orientation, chosen from the face it holds.
- `fruit-def` gains `shape: option<string>`: the Shape of fruit Voxels, so the
  approved 3D voxel apple is a box Shape. `tree_voxels` returns a Shape and
  Orientation with each Voxel, and growth and worldgen place them with it.
- `tree-def` trunk and canopy stay cubes unless a Shape is named later.
- Terra and `flora` declare their `cross` and `plane` Shapes once in `flora` and
  every plant names its own.

## Acceptance criteria

- [x] A seed placed from the Hotbar is the plant's Shape whatever the Player
      selected; a Layout of seeds likewise, all through Place and Layout.
- [x] Growth, spreading and generation place the Plant's Shape; a Stage change
      keeps it.
- [x] A tree's fruit Voxels are a box Shape (the 3D apple) in a grown tree and in
      a generated one, whole across Chunk borders.
- [x] A Plant that names no Shape keeps today's behaviour; an unknown Shape is a
      load error naming the Plant.
- [x] The save needs nothing: a Voxel is Material, Shape and Orientation.
- [x] Worlds that declare no Plant features generate the recorded digests.

## Comments

- `plant-def.shape` and `fruit-def.shape` are optional full Shape names. An
  unknown name is a load error naming the Plant. `PlantTable::placed` is the one
  rule for a seed: the Plant's Shape and, for a `plane`, the Orientation holding
  the face the Player struck; the Player's Shape and Orientation are ignored.
  The Server applies it in `resolve` (so PlaceHeld) and once per Layout in
  `drain`, where the aimed face is read once and a Layout between corners keeps
  the Shape's own Orientation.
- Only a Plant's first Stage, sapling or single Material is placed this way.
  Growth already copied the growing Voxel's Shape, so Stage changes, spread,
  columns and climbers keep it with no change; `tree_voxels` now returns each
  Voxel's Shape and Orientation and `sprout` places them.
- `CatalogueMaterial.shape` now reports a Plant seed's Shape as a Door's fixed
  Shape is reported. The Orientation depends on the aimed face, so
  `AimGhost.plant_orientation` carries it and the Client ghost draws that.
  Ticket 13 draws the flat sprite and keeps that field.
- Worldgen: the Biome feature's `shape` stays as an override of the Plant's own
  (a tree still refuses one); with none the Plant's Shape is used, so digests of
  Worlds with no Plant features are unchanged. `SurfaceFruit.shape` is frozen
  by name as `SavedFruit.shape` (serde default, absent is a cube);
  FORMAT_VERSION and GENERATOR_VERSION are unchanged.
- Terra and `flora`'s plants name `cross`/`plane`; Terra's feature rows no
  longer repeat the Shape. Terra's `apple` Shape is four boxes (body 10/16 wide,
  walked through, seals no face). One Shape serves both fruit Stages, so the
  unripe apple is the same size as the ripe one and the underside is not darker:
  a Shape has no colour, only the Material has.
- `server::plants::a_region_over_the_plants_size_in_metres_does_not_break_and_a_small_one_does`
  fails about one run in eight when the module runs in parallel, with or without
  these changes; it passes alone.
- `cargo test -p tesserae-mods` fails 26 tests that read `target/guests` (no
  `hello.wasm` here); the plant, starter and data tests pass.
