# 08: The Server places a Structure for a Creative Player

Status: ready-for-agent

Blocked by: 02

Spec: [Phase 20](../spec.md).

## What to build

A Structure intent names a Template, an anchor from the aim ray and a quarter
turn. The Server expands the cells, judges them as a whole and lands them
through the one World change in per-tick batches with one relight per batch.
Only the anchor must be in reach. It is Creative only and spends no Item. It
waits a bounded number of ticks for unloaded or unlit Chunks, then refuses with
a reason, and it lands whole or not at all. Mods hear the Voxels as Edit events.

## Acceptance criteria

- [ ] A Structure larger than the edit reach lands whole when its anchor is in
      reach.
- [ ] Survival refuses with a reason, and Creative spends no Item.
- [ ] A placement over an unloaded Chunk waits, then lands whole or refuses
      after the bound.
- [ ] A refusal leaves the World unchanged.
- [ ] A placed Structure is saved like any build and reloads.
- [ ] The Edit events reach a Mod.
- [ ] A Structure at 0.25 m lands without blocking the tick beyond the existing
      per-tick budget.
