# 17: Plants carry their Shape

Status: ready-for-agent

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

- [ ] A seed placed from the Hotbar is the plant's Shape whatever the Player
      selected; a Layout of seeds likewise, all through Place and Layout.
- [ ] Growth, spreading and generation place the Plant's Shape; a Stage change
      keeps it.
- [ ] A tree's fruit Voxels are a box Shape (the 3D apple) in a grown tree and in
      a generated one, whole across Chunk borders.
- [ ] A Plant that names no Shape keeps today's behaviour; an unknown Shape is a
      load error naming the Plant.
- [ ] The save needs nothing: a Voxel is Material, Shape and Orientation.
- [ ] Worlds that declare no Plant features generate the recorded digests.
