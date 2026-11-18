# 18: Biome and field tests measure what they name

Status: ready-for-agent

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

- [ ] The bare-Biome test runs under version 3 and passes, still failing for a
      Biome that grows on its own surface by accident (a test case shows it).
- [ ] The fields test counts field plots only, and still passes for every Game.
- [ ] No Engine code or Game content changes.
