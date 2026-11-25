# 08: Coarse trees stand near the spawn while it loads

Status: done

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

- [x] Reproduced on a capture from a new World's first seconds.
- [x] No tree near the Player draws in coarse Voxels while level 0 loads, and
      no gap or black shows instead (06 stays fixed).

## Comments

2026-11-25: the user adds that it happens on every load, also when returning to
a saved World, not only a new one. Captures at the default spawn 2 s after
load show level 0 alone near the eye, with no coarse level drawn yet and no
giant leaves. If 06 is the cause, the coarse trees would come with the coarse
levels after those first seconds and leave within 10 to 15 s; not yet seen.

2026-11-26: cause and fix (Q105, Q106).

Reproduced on a copy of the user's World (seed 16752804544220520129, preset
High, reach 7 km) with a temporary shutter at a fixed time after start; with
`TESSERAE_LOD_TINT=1` the blocks drew in level 5's colour. Per level at 12 s:
level 0 done at 3.2 s, level 5 at 7.2 s, levels 1 to 4 barely begun (level 1
16 of 3084 meshed). Level 5 then stands in for all of levels 1 to 4 (06,
Q61), and its trees, in 32 m Voxels, stand from about 160 m out: one leaf
block reads as a whole tree. Not an air-coverage fault: finer levels' empty
Chunks are sent and marked drawn as before. Levels 1 to 4 are about 27 000
Chunks, about three quarters of them sky or solid rock, generated coarse to
fine, so level 1 starts last; nothing coarse is saved with a World, so a
return regenerates them all.

Fix (A): a level meshes a Chunk it stands in for without its Plants, its
neighbours too, so the ground still fills the gap and no tree shows in Voxels
made for its own distance. Such a Chunk is meshed again with its Plants if
the anchor moves it out of the hole before finer levels cover it. On the
captures at 10 and 15 s the land past level 0 shows bare instead, and its
trees arrive with levels 1 to 4 by 25 s. Test:
`a_stand_in_draws_its_ground_and_none_of_its_trees`. Shortening the wait is
`.scratch/2026-11-26-coarse-level-load-time/`.
