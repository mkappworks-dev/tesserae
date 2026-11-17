# 02: Incremental Commits

Status: ready-for-agent

Blocked by: 01

Spec: [Save: retention, incremental commits and binary Chunk records](../spec.md).

## What to build

Format 3. A Commit is a `<seq>.commit` manifest (`TESSAVE1`, SHA-256, JSON with `"format": 3`) naming each Chunk and paused simulation share by a record reference; records first needed by Commit `seq` go in an immutable `<seq>.pack`. The Writer remembers the newest manifest's references, so the Server captures and encodes only Chunks whose revision it does not already store and paused shares that are new handles. Garbage collection deletes packs no Commit on disk names; compaction copies the records of a pack the newest manifest uses less than half of into the next pack. Paused shares stop repeating the active simulation's fields. A Chunk with no revision whose only pending work is rescans is evicted without a Commit and its rescans dropped, since `admit` queues them again. Records may stay today's encoding (deflated JSON) until ticket 03.

## Acceptance criteria

- [ ] Editing one Chunk after a save writes a pack with one Chunk record, however many Chunks the World has changed.
- [ ] An unchanged paused share is never written twice; a share's restore is unchanged (`edits_removals_fluid_fire_and_paused_timers_survive_eviction_and_restart` and the agenda and timer round trips pass).
- [ ] An unchanged wet Chunk evicted mid-rescan leaves without a Commit or paused entry and settles the same when it returns.
- [ ] Format 1 and 2 Commits open; the first write after them stores every Chunk.
- [ ] A manifest with an unknown format is refused as incompatible, never skipped as corrupt; a record failing its hash makes its Commit corrupt.
- [ ] Error injection covers the pack's body, sync and rename; a kill between the pack's rename and the manifest's recovers the previous Commit, and the next writer sweeps the orphan pack.
- [ ] A pack a kept Commit names is never deleted; compaction moves records and the old pack ages out.
- [ ] `acquire` reserves `.pack` sequences.
- [ ] Save time per Commit and the cost of the full flushes are recorded below.
- [ ] `docs/world-persistence.md` describes format 3, packs, garbage collection and compaction.
