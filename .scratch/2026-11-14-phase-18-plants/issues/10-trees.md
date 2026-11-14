# 10: Trees

Status: ready-for-agent

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

- [ ] A sapling with room grows a tree of the declared metres at 1.0, 0.5 and
      0.25 m; with any cell blocked it stays a sapling and tries again.
- [ ] A tree with `fruit` places fruit Voxels at hash-chosen canopy positions
      within its density; they ripen by the Sample tick and are picked again
      after `harvest-to`.
- [ ] The group lands whole or not at all, relit once, under the per-step bound.
- [ ] Undo of the seed's placement after the tree grew is refused by the
      journal's mismatch rule; redo and a pre-growth undo are unaffected.
- [ ] Trunk and canopy are grown Materials: no implicit Item, not placeable,
      not in the Catalogue; removing one drops its declared yield, never a grown
      Material.
- [ ] Harvest and Remove of the sapling drop its declared yield.
- [ ] The tree shape is a pure function of the row, a position in metres and
      the Voxel size, tested without Bevy.
