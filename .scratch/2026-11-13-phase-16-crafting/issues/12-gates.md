# 12: Gates

Status: done

Blocked by: 09, 11, 13

Spec: [Phase 16](../spec.md), amendment 2026-11-14.

## What to build

A smaller Door for fences. Terra declares a Gate: a `door` row 1 m wide by 1 m
tall, with `gate` and `gate_open` Materials, a Gate Item, and a Workbench Recipe
(2 Wood). It places, toggles, breaks and undoes exactly as ticket 09's Doors do,
and uses the door Causes for sound. Its look matches Terra's fences: closed,
horizontal rails between two stiles; open, swung flush to one edge. The look
goes through the `/blocks` gallery for the user's approval before it is
committed, beside a fence run and the 1.8 m Player silhouette.

## Acceptance criteria

- [x] A Survival player crafts a Gate at a Workbench, places it in a fence line,
      opens it, walks through and closes it.
- [x] At 1 m and 0.25 m Voxels a Gate is 1 m by 1 m.
- [ ] The gallery shows the Gate closed and open beside a fence; the user
      approves the look before the Terra commit.
- [x] Terra's Recipe count, Material count, texture fingerprint and appearance
      pins are updated.

## Decisions

- The Gate is Terra content only: a `door` row of 1 m by 1 m with the fixed
  `terra:panel` Shape, so placing, Use, breaking, undo and the door sound Causes
  are ticket 09's and 13's unchanged.
- Its look is cut out, not modelled: `gate` and `gate_open` are Cutout Materials
  that occlude nothing and dim no light, and the panel's side texture is two
  stiles and three rails, the upper two at the edges of a fence's rail, with the
  rest transparent. Top and bottom show only the stiles' ends, in the panel's
  corners, which holds whichever edge of the cell the panel stands on.
- The look commit is separate and last, for the user's approval in `/blocks`.
  The gallery shows every Door in its own section; one shorter than the Player
  stands in 2 m of wooden fence either side.
- Deferred: a closed Gate stands on its cell's near face, so it sits about 5/16
  of a Voxel off a fence's centre line, and a fence beside it does not join it.
  At finer Voxels each cell repeats the whole stiles-and-rails texture.
