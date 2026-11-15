# 10: Trees

Status: done

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

The `tree` kind: a planted sapling grows the parametric tree, a trunk of
`trunk-height-m` (hash-chosen within its range) and an ellipsoidal canopy of
`canopy-radius-m`, in metres, as one all-or-nothing group of changes under
`TREES_PER_STEP`. Every cell must be Air or already the tree's own Materials,
or the group does not land and the sapling Samples again. The pure shape
function is shared with worldgen (ticket 12).

## Acceptance criteria

- [x] A sapling with room grows a tree of the declared metres at 1.0, 0.5 and
      0.25 m; with any cell blocked it stays a sapling and tries again.
- [x] A tree with `fruit` places fruit Voxels at hash-chosen canopy positions
      within its density; they ripen by the Sample tick, are broken to canopy, and
      the standing tree grows new unripe fruit elsewhere on its canopy.
- [x] The group lands whole or not at all, relit once, under the per-step bound.
- [x] Undo of the seed's placement after the tree grew is refused by the
      journal's mismatch rule; redo and a pre-growth undo are unaffected.
- [x] Trunk and canopy are grown Materials: no implicit Item, not placeable,
      not in the Catalogue; removing one drops its declared yield, never a grown
      Material.
- [x] Harvest and Remove of the sapling drop its declared yield.
- [x] The tree shape is a pure function of the row, a position in metres and
      the Voxel size, tested without Bevy.

## Comments

- The tree is `plant/tree.rs::tree_voxels(&Tree, seed, base, voxel_size) ->
  Vec<TreeVoxel { at, material, part }>`, `base` being the sapling's cell. The
  trunk height is a whole number of Voxels inside `trunk-height-m` chosen by a
  hash of seed and the base in millimetres; the canopy is an ellipsoid (vertical
  radius 0.8 of `canopy-radius-m`, never below half the trunk, never at the
  ground) centred on the trunk top; `bears_fruit(seed, density, at, size)` picks
  fruit Voxels by hash of seed and position, and a fruit Voxel is the canopy's
  place at fruit Stage 0.
- `grow` returns one `Growth { tree: true }` for a sapling and the Server lands
  it under `TREES_PER_STEP = 2`, apart from `PLANTS_LANDED_PER_STEP`. Every cell
  is checked in `grow` (Air, the tree's own grown Material, the sapling's own
  cell, known and lit) and `land` also puts back what a failed write had
  written, so a tree is whole or absent. One `Batch` relights the group once.
- Fruit ripens through `Role::Fruit(n)` on the tree's light and Climate, with
  light read as the brightest of the Voxel and its six neighbours, since fruit
  sits inside canopy. Breaking fruit is an ordinary Remove (canopy is left; a ripe one drops its
  yield).
- Re-bearing: each canopy Voxel of a fruit tree Samples at a mean of
  `regrow-seconds * N`, with `N` the expected canopy cell count
  (`tree::expected_canopy_cells`, the mean over four reference trees, cached per
  Plant and Voxel size), so the tree rolls one passing Sample every
  `regrow-seconds`. A pass becomes unripe fruit if light and Climate hold and
  the fruit within twice `reach_m` (spherical, `voxels_in`) is below `density *
  N`; it lands as a plain growth under `PLANTS_LANDED_PER_STEP`. A Voxel queued
  for Felling is skipped, and a felled tree has no canopy left.
- Undo of the seed after growth was already refused by the mismatch rule
  (`undoing_a_sapling_placement_after_it_grew_is_refused`).
- `TREES_PER_STEP` was measured by the benchmark pass: an oak of 4 to 6 m with
  a 2.5 m canopy lands in 0.16 ms at 1.0 m (52 Voxels), 0.86 ms at 0.5 m (420)
  and 6.1 ms at 0.25 m (3341), whole group and relight. At 2 a step the last is
  12 ms of a 15.6 ms step, so the bound is now 1.
