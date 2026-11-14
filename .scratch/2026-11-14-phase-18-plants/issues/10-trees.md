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
      within its density; they ripen by the Sample tick and are picked again
      after `harvest-to`.
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
  sits inside canopy. Picks reset through the existing harvest path.
- Undo of the seed after growth was already refused by the mismatch rule
  (`undoing_a_sapling_placement_after_it_grew_is_refused`).
- `TREES_PER_STEP` is a starting value; the bench of ticket 04 has not measured
  it.
