# 02: Incremental Commits

Status: ready-for-agent

Blocked by: 01

Spec: [Save: retention, incremental commits and binary Chunk records](../spec.md).

## What to build

Format 3. A Commit is a `<seq>.commit` manifest (`TESSAVE1`, SHA-256, JSON with `"format": 3`) naming each Chunk and paused simulation share by a record reference; records first needed by Commit `seq` go in an immutable `<seq>.pack`. The Writer remembers the newest manifest's references, so the Server captures and encodes only Chunks whose revision it does not already store and paused shares that are new handles. Garbage collection deletes packs no Commit on disk names; compaction copies the records of a pack the newest manifest uses less than half of into the next pack. Paused shares stop repeating the active simulation's fields. A Chunk with no revision whose only pending work is rescans is evicted without a Commit and its rescans dropped, since `admit` queues them again. Records may stay today's encoding (deflated JSON) until ticket 03.

## Acceptance criteria

- [x] Editing one Chunk after a save writes a pack with one Chunk record, however many Chunks the World has changed.
- [x] An unchanged paused share is never written twice; a share's restore is unchanged (`edits_removals_fluid_fire_and_paused_timers_survive_eviction_and_restart` and the agenda and timer round trips pass).
- [x] An unchanged wet Chunk evicted mid-rescan leaves without a Commit or paused entry and settles the same when it returns.
- [x] Format 1 and 2 Commits open; the first write after them stores every Chunk.
- [x] A manifest with an unknown format is refused as incompatible, never skipped as corrupt; a record failing its hash makes its Commit corrupt.
- [x] Error injection covers the pack's body, sync and rename; a kill between the pack's rename and the manifest's recovers the previous Commit, and the next writer sweeps the orphan pack.
- [x] A pack a kept Commit names is never deleted; compaction moves records and the old pack ages out.
- [x] `acquire` reserves `.pack` sequences.
- [x] Save time per Commit and the cost of the full flushes are recorded below.
- [x] `docs/world-persistence.md` describes format 3, packs, garbage collection and compaction.

## Comments

- Paused shares store only their timers, wakes, rescans and landing cursors; a share restores with the active simulation's bounds and turn rather than storing them when equal, since resuming reads none of them but the turn, and that only as a fallback.
- A Writer names a paused share again only while it is the same handle, so the first write after an open stores every share again.
- The first flight still paused 3,042 shares, 2,589 of them rescan-only shares of Chunks never stored: an eviction that waited for a Commit (a running step reading the Chunk, say) went through `pause_evictions`, which kept the rescans. That path now drops them too (`an_unchanged_chunk_paused_mid_rescan_saves_no_share_and_settles_on_return`, which fails without it); the next flight paused 463, every one of a stored Chunk.
- **Flight** (2026-11-18, on main `b88e7c12`, load 5.7 to 6.5): save worker time per Commit 93 ms median, 256 ms max, against 590 to 599 ms and 970 to 1,028 ms on main. Bytes written per Commit 524 KB median: a 367 KB manifest and a 166 KB pack. Full table in ticket 03.
- **Full flushes.** `sync_all` is `F_FULLFSYNC` on macOS: 3.9 ms median for a 92 KB pack and 3.8 ms for a 365 KB manifest, against 0.03 and 0.07 ms for a plain `fsync`, measured on the same volume. About 8 ms of a 93 ms save, the pack's share 4%, so the pack keeps its full flush.
- The guests built, the whole Server suite passes, the five demo-game tests among them.
