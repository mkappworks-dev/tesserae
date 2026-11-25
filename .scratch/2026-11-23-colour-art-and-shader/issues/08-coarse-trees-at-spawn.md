# 08: Coarse trees stand near the spawn while it loads

Status: needs-triage

Spec: [Colour, art and light](../spec.md).

## What happens

Reported by the user on 2026-11-25, playing this branch: entering a new World
whose spawn is in a Settlement, the trees near the Player show as giant leaf
blocks, each about a tree tall and wide. They go away after 10 to 15 s. Seen
on this branch only; not reported on `main`.

## Likely cause (not confirmed)

The size fits a coarse level's Impostor: a level 3 Voxel is 8 m, about a
crown. Ticket 06 (Q61) keeps a coarser level drawing a region until every
finer Chunk covering it is sent, so no gap shows beneath; while level 0 near
the Player is still loading, the coarse level therefore draws the nearest
trees as Impostors, metres away. On `main` that region showed a gap or black
instead.

Not reproduced in captures yet: the capture waits until the spawn
neighbourhood is held and lit, and at the default spawn (from 64 m and at eye
height, four headings, 6 s and 40 s settle) every tree was already level 0.
The user's World is a new random seed, which could not be copied (see the
copy-seed ticket in `2026-11-25-copy-seed`).

## To decide in triage

- Whether a coarse level should draw Impostors (and canopy cover) inside
  level 0's ring at all while it stands in, or only the ground.
- Or whether level 0 around the spawn should be sent before the Player sees
  the World, as the capture's wait already does.

## Acceptance

- [ ] Reproduced on a capture from a new World's first seconds.
- [ ] No tree near the Player draws in coarse Voxels while level 0 loads, and
      no gap or black shows instead (06 stays fixed).

## Comments
