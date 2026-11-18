# 13: Fix version 3 generation bugs found while moving tests

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 09.

## What to build

Three version 3 defects found by tickets 02 and 03 (see their Comments) are
fixed inside the one pre-release exception ADR-0032 allows, before 12 re-records
the digests:

1. A Surface feature on a column's top Voxel at a Chunk's top can go missing
   from the Chunk above, because that Chunk asks the raw tunnel field, which
   does not know version 3 keeps tunnels shut beside the sea.
2. In ticket 03's scan area, 1 m Voxels place crop fields near water and 0.5 m
   Voxels place none. Find out why; if placement depends on Voxel size rather
   than metres (ADR-0003), fix it.
3. At seed 3907 with overhangs on, smoothing places stairs that climb two steps
   and slabs that ease fewer than two.
4. A landform height (`landforms.height_m`) of 0 divides by zero: heights turn
   NaN and drainage loops forever, so a Game declaring it hangs the Server
   (found by ticket 16). Validation refuses it with a clear error; no valid
   World's output changes.
5. Version 3 drops a tree within 1.5 crown radii of a stronger one, which caps a
   stand of one tree far below the dense density class (ticket 17's Comments:
   jungle 57 per hectare against 278). Each Plant declares its own crowding
   distance, defaulting to today's rule, so a dense Biome's trees can stand
   closer; the Games' dense trees declare one that reaches the class.

## Acceptance criteria

- [ ] A test per defect fails before the fix and passes after it, at 1 m and 0.5
      m.
- [ ] Ticket 03's raised-sea case in the Surface feature test is wet again, and
      the smoothing tests also run at seed 3907 with overhangs.
- [ ] The digest and spawn positions each fix moves are listed in a comment
      here; they are re-recorded in this ticket, and 12's commit message lists
      them alongside its own.
- [ ] If item 2 turns out not to be a defect, the reason is written here and the
      test asserts the intended behaviour.
- [ ] Parameters with a landform height of 0 (or any value that makes a height
      non-finite) are refused at validation, shown by a test; no digest moves
      for this item.
- [ ] `each_biome_scatters_its_trees_at_its_density_class` runs under version 3,
      unignored, with its thresholds unchanged.
- [ ] A Plant that declares no crowding distance crowds exactly as before; the
      new field is optional in the Declaration and documented where Plant fields
      are (contract compatibility, data mods).
