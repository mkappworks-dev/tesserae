# 31: Placed Structures sit level with the ground

Status: done

Blocked by: 08, 14

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found every placed Structure's doors one Voxel above the ground
outside.

- The place Tool's anchor is the cell in front of the face the ray strikes,
  and the Placement puts the Ground layer's lowest Voxel layer on it. Aimed at
  the ground's top face, that is the air above the ground, so the floor stands
  one Voxel up. Worldgen puts the Ground layer on the surface Voxel instead.
- At finer Voxels both are off: the Ground layer is a 1 m cell, so with its
  lowest Voxel layer on the surface Voxel its top stands up to 1 m less one
  Voxel above the ground, 0.5 m at 0.5 m and 0.75 m at 0.25 m.

Fix both so the Ground layer's top Voxel layer replaces the ground's top Voxel
layer: a floor and a door's sill are level with the ground outside at 1.0, 0.5
and 0.25 m, from the place Tool and from worldgen alike. A Template whose
cells are on whole metres still lands on whole metres.

## Acceptance criteria

- [x] Placing a Template aimed at flat ground puts its floor's top level with
      the ground's top at 1.0, 0.5 and 0.25 m, tested through the Server.
- [x] A worldgen Structure and a Settlement building stand with their floor's
      top level with the ground beside them at every size, tested.
- [x] The ghost still shows what then lands.
- [x] Worlds without Structures stay byte-identical.

## Comments

Where it lives: `Sampled::ground_top` (`template.rs`), the highest Voxel layer
of the Ground layer; worldgen's `Row::ground_top` (`worldgen/structures.rs`)
and `crossings.rs`, and `plot` and the pass bounds in
`worldgen/settlements.rs`, stand it on the ground's top Voxel layer;
`structure_anchor` takes the struck Voxel's own layer (the cell in front of the
face across, and the layer below under a ceiling), and `Placement::new` puts
`ground_top` on it.

Decisions:

- The place Tool aimed at the ground's top replaces that layer, as worldgen
  does. Aimed at a wall's side the Ground layer's top is the struck Voxel's
  layer.
- At 1 m nothing moves in worldgen (`ground_top` equals `ground` there), so the
  1 m digests stand. At 0.5 m and 0.25 m worldgen Structures, Settlement
  buildings and bridges stand 0.5 m and 0.75 m lower than before, their floors
  level with the ground; no recorded digest covers them.
- The Ground layer's Ground cells still take the surface Material from the
  Ground layer's lowest Voxel layer up.

Tests: `a_placed_floor_is_level_with_the_ground_it_is_aimed_at_at_every_voxel_size`
(Server); `a_cellar_is_underground_and_the_floor_level_with_the_surface` now asks
for the floor's top on the median top; `every_plot_is_level...` asks each
Plot's floor top to equal its apron's median top. Tests that aimed at the
ground with a one-layer Template gained a Keep Ground layer under it, and the
undo-budget test's hall grew from 32 to 48 m, as the floor no longer counts
when it replaces stone with stone. The bridge test measures its gap from the
deck's top Voxel layer.
