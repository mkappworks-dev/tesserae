# 11: Felling

Status: done

Blocked by: 06, 10

Spec: [Phase 18](../spec.md).

## What to build

Cutting a trunk breaks what depended on it. When a trunk Voxel of a `tree` Plant
changes, the Server wakes its neighbours and evaluates the Felling region: the
grown Voxels of that tree connected through faces that no longer reach a
grounded grown trunk (one standing on a Voxel that is not grown), bounded by the
Plant's declared size in metres. Only grown Materials take part, so what a
player placed is never felled, whatever it looks like. A region over the size
bound does not break. It breaks top-down over successive Logical
steps under `FELLED_PER_STEP`, each Voxel dropping its Item like any removal. The
cascade is not an Edit the journal records; undo restores the cut Voxel only.
Falling as a body, in a direction, is step 24's.

## Acceptance criteria

- [x] Cutting a grown tree's trunk breaks the part above the cut over several
      steps, top-down, never more than `FELLED_PER_STEP` per step, and drops
      Items.
- [x] A felled tree's fruit Voxels drop their Stage's yield; unripe fruit drops
      only what its yield declares.
- [x] Every Felling drops at least one sapling, once for the whole tree and not
      per Voxel, picked up and planted to grow another tree; no sapling drops
      when the cut leaves the tree grounded.
- [x] A cabin of placed logs and leaves, and a placed leaf touching a grown
      trunk, are never felled; a region over the size bound is not either.
- [x] Cutting a trunk Voxel that leaves the tree grounded breaks nothing else.
- [x] Burn-out and a Mod Command's removal fell by the same rule.
- [x] Undo after a felling restores the Voxel the Player cut and no more; the
      journal's group count is one.

## Comments

- `plant/tree.rs::felling_region` is the pure rule: a search from one grown tree
  Voxel over that Plant's trunk, canopy and fruit Materials, grounded when a
  trunk Voxel stands on a Voxel that is neither Air nor any Plant's. A region
  wider than `2 * canopy-radius + 3` Voxels or taller than `trunk-height-max +
  canopy-radius + 3` does not break, and neither does one beside a Voxel the
  Server does not hold.
- The Server asks about the cells beside every change in `absorb_changes` and
  `wake_landed`, so Player Remove, Mod Commands, burn-out and the Felling's own
  writes all reach it by the one write path. A Felling queues its Voxels in
  `PlantSim::felling` and `fell_step` breaks `FELLED_PER_STEP = 32` of them a
  Logical step, highest first, through `Batch::set_occupants` with no journal
  entry. Nothing is polled: an empty queue returns at once.
- The saplings (`sapling-yield`, at least 1, a hash of seed and the cut cell)
  drop once, when a region is first queued, whenever the region is not empty.
  That includes a region with no trunk Voxel (the canopy above a cut at the
  trunk's top); a leaf cut that leaves the tree grounded drops none.
- A pending Felling is not saved: a World closed mid-fall leaves what was
  queued standing until a neighbouring change asks again.
- Burn-out is tested through the `set_occupants` it lands by, not through the
  fire simulation. `FELLED_PER_STEP` was measured by the
  benchmark pass and stays 32: a step costs 0.11 ms at 1.0 and 0.5 m and 0.38 ms
  at 0.25 m (a 3340-Voxel oak). The relight and the wake are paid once a step,
  so a Voxel costs 13 us at 32 a step and 3.3 us at 512; 128 a step would be
  0.54 ms at 0.25 m, so there is headroom.

