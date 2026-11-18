# 18: Biome and field tests measure what they name

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

Two Game tests measure more than their subject (ticket 17's Comments):

- `every_biome_…_none_is_bare_by_accident` counts what a Biome grows in its
  caves and fruit hanging from a neighbour's Plant. It counts only the Biome's
  surface layer, and tolerates a neighbour's fruit as it already tolerates a
  neighbour's limbs and vines. It then runs under version 3.
- The fields test's comparison World drops the Biome selectors, so under version
  3 it also counts scattered crops and Settlement farm Templates. It compares
  against a World that keeps everything but the field plots, so it measures
  field plots alone.

## Acceptance criteria

- [x] The bare-Biome test runs under version 3 and passes, still failing for a
      Biome that grows on its own surface by accident (a test case shows it).
- [ ] ~~The fields test counts field plots only, and still passes for every
      Game.~~ Moved to 19.
- [x] No Engine code or Game content changes.

## Comments

- Needs a decision: the fields criterion is not met. Measured against a World
  that keeps every Biome's selectors, strata, Structures and Settlements and
  drops only the crop fields, Terra's census holds three field plots, wheat and
  potato, with no carrot plot and no ditch: at seed 7 within 512 m of spawn,
  within 1024 m (wheat 188, potato 87 Voxels), and within 512 m of the temperate
  ground nearest spawn, beside water or not. Terra plants fields only in
  temperate (5.6% of the land), every 72 m within 40 m of water or every 112 m,
  carrot 2 of 10. Its version 3 pass came from scattered crops and Settlement
  farms. Meadow, tidewater, lumen and ember pass at their windows with field
  plots alone (meadow wheat 353, carrot 113, potato 167; tidewater 52, 28, 23
  and a ditch; lumen 601, 90, 99; ember potato 240, carrot 182, wheat 148 and a
  ditch). The fix is Terra's field spacing or carrot weight, a Game change; the
  fields test is unchanged here.
- The bare-Biome test counts a surface census: a Voxel counts only at or above
  its column's surface less a Voxel, so the mushroom grotto under Terra's alpine
  is left out (the full census holds 510 brown and 294 red mushrooms there).
- Two more failures under version 3 than ticket 17 found, both of the census,
  not the Games:
  - Tidewater's sample held no water plants: a Biome's water is a few lakes (0.4
    to 1.3% of its columns), which the first 32 sampled Chunk columns all
    missed. The census now also samples up to 8 Chunk columns per Biome whose
    centre stands under water; tidewater's lakes then hold seagrass, kelp and
    red weed in every Biome that declares them.
  - Tidewater's metre-wide shore held 112 swamp-log Voxels against the old limit
    of 100, every one a limb 7 to 14 m up from about ten tidal-marsh trees: the
    limit grew with the border's length. A bare Biome now tolerates a
    neighbour's logs, vines and fruit in any number and fails on a trunk foot: a
    log on the ground, not on a Plant or water, under 4 m (`TREE_CLEAR_M`) of
    logs. The 4 m tells a trunk from a pine limb resting on alpine's slope, two
    logs high.
- `a_bare_biome_that_grows_on_its_own_surface_is_caught` gives Terra's beach the
  temperate Biome's features and finds tufts and a trunk foot there.
