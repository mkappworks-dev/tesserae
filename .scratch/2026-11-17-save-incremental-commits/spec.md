Status: done

# Save: retention, incremental commits and binary Chunk records

Reduce what a World's save writes and keeps. Builds on format 2 (each Chunk a palette and deflated indices, commit `7987f202` on step 17's branch); this branch merges after step 17's.

Vocabulary: **Commit** is new in [GLOSSARY.md](../../GLOSSARY.md). The decision not to store Chunks as deltas from World generation is [ADR-0031](../../docs/adr/0031-a-saved-chunk-is-stored-whole-not-as-a-delta-from-generation.md); its follow-up is [generator determinism](../2026-11-17-generator-determinism/spec.md). The contract is [docs/world-persistence.md](../../docs/world-persistence.md). Related: [ADR-0013](../../docs/adr/0013-fluid-and-fire-live-beside-the-voxel.md), [ADR-0022](../../docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md).

## Problem Statement

A 3-minute hands-off flight in a fresh version 3 World writes 99 Commits of about 2 MB each and keeps all of them: 195 MB of disk. Three causes:

- Every Commit is kept forever ("no garbage collection yet").
- Every Commit holds every Chunk the World has ever changed, loaded or evicted, re-encoded each time and then inflated again to validate it. Commits come every few seconds while moving, because a changed Chunk must be committed before it can be evicted.
- `paused`, the largest section (1.9 MB of 2.8 MB), copies every evicted Chunk's simulation share into every Commit until that Chunk returns, spelling each flow timer's Material name and four levels in JSON. In the baseline run below, 4,300 of its 4,333 entries (2.2 MB) are an unfinished arrival rescan of a Chunk the save never stored: one never changed, evicted before its rescan ended. Each also held its Chunk back from eviction until a Commit covered it.

## Solution

Three changes, one commit each:

1. **Retention.** Keep the three newest Commits this writer completed and delete older ones only after the newest is durable.
2. **Incremental Commits.** Store each Chunk version and each paused simulation share once, in an immutable pack written with the Commit that first needs it. A Commit's file becomes a manifest naming the records it uses, so a Commit costs only what changed since the last one. Records no kept Commit names are deleted; packs mostly unreferenced are compacted forward.
3. **Binary records.** A pack's records are binary: a record-local name table, a palette, bit-packed indices, deflated. `tesserae save dump` prints a Commit as JSON.

## User Stories

1. As a Player, a World I fly through for an hour uses megabytes of disk, not gigabytes.
2. As a Player, autosaving while I move does not grow with how much of the World I have ever changed.
3. As a Player, a crash or power cut mid-save still reopens my last complete save.
4. As a Player, an older Tesserae refuses a World saved by a newer one and leaves it untouched, rather than opening an older save and writing over the newer.
5. As a Player, a World saved in format 1 or 2 opens and keeps its Chunks.
6. As a developer, I can print any Commit of a World as JSON to see what it holds.

## Implementation Decisions

**Retention (ticket 01).**

- `KEPT_COMMITS = 3`. After a write whose whole file protocol completed, the Writer deletes every `.commit` older than the third-newest Commit it completed since `acquire`. Before its third, it deletes nothing. A corrupt or reserved sequence sits below the new ones, so it ages out the same way.
- A failed delete is a warning the Server logs, never a save failure; the next write retries. Deletion order does not matter for safety: recovery already falls back newest first.
- `acquire` deletes leftover `.pending` files while holding the lock.
- No hourly retention. One constant would add it later.

**Incremental Commits (ticket 02).**

- **Format 3.** One bump for the branch, set in ticket 02 and refined in place by ticket 03; nothing ships between them. Formats 1 and 2 still open.
- **Files.** `<seq>.commit` is the manifest: the `TESSAVE1` magic, the SHA-256 of its body and a JSON body with `"format": 3`. It keeps that shape so every engine that knows only formats 1 and 2 refuses it as an unsupported format. An unreadable file would be skipped as corrupt and an older Commit reopened and written over. `<seq>.pack` holds the records first written by Commit `seq`; engines before format 3 never open it.
- **Manifest.** Everything a Commit holds today, except that `chunks` names each Chunk by coordinate, revision and a record reference (pack sequence, offset, length, SHA-256 of the record bytes), and `paused` names each home and its share's record the same way.
- **Pack.** A `TESPACK1` magic, then records back to back. The manifest is the index; a record is verified against its hash when read.
- **Write protocol.** Pack to a unique `.pending`, sync, rename to `<seq>.pack`; manifest to a `.pending`, sync, rename to `<seq>.commit`; sync the directory once, which covers both renames. A Commit with no new record writes no pack. `acquire` reserves sequences of `.pack` files as well as `.commit` files.
- **What a Commit writes.** The Writer remembers the newest manifest's references. A Chunk whose coordinate and revision it already stores is named, not captured or encoded; a paused share is named while it is the same immutable handle. Only new records are validated on write; at open every referenced record is read, verified and restored, as today.
- **Garbage collection.** After retention, if every `.commit` on disk parses, delete each `.pack` none of them references. A missing file is success, so a crash only leaves an orphan for the next run. If any manifest does not parse, skip the sweep.
- **Compaction.** When the newest manifest references less than half of an older pack's record bytes, the next Commit copies those records byte for byte into its own pack and names them there, so the old pack ages out with retention. Disk stays within about twice the live records.
- **Rescans of unchanged Chunks are dropped, not paused.** A Chunk with no revision whose only pending simulation work is rescans is evicted at once, its rescans discarded, as dry terrain is: `admit` queues them again when it returns. A rescan cut short restarts from the beginning; the Chunk settles the same.
- **`paused`.** A share is stored once and named until it changes. The per-entry fields that `Simulation::empty` copies from the active simulation are not stored when they equal the active one's; fluid Material names go through the record's name table in ticket 03.
- **First write after opening an older format** stores every Chunk, since the Writer holds no references.
- **Sync.** `sync_all` stays (a full flush on macOS). Ticket 02 records its cost per Commit; replacing the pack's full flush with a plain `fsync` ordered by the manifest's full flush is done only if that cost shows in save time.

**Binary records (ticket 03).**

- A record starts with a kind (Chunk or paused share) and a record version. Its body is deflated at the default level: a record-local name table (each Material and Shape name once), then the content. A record is self-contained, so compaction copies bytes.
- **Chunk record.** The palette (solid, fluid, fuel, by name-table index), then one index per Voxel in x/y/z order, z fastest, packed at `ceil(log2(palette))` bits; a single-cell Chunk has none. Byte planes against bit-packing is measured on the baseline World's real Chunks and the smaller kept; both numbers go in the ticket.
- **Paused share record.** Timers, wakes, rescans and landing cursors, binary, Materials by name-table index.
- **No zstd.** On a Chunk-shaped 64 KiB buffer, `ruzstd` 0.9.1's only encoder level produced 18,983 B against 2,730 B for today's deflate-fast and 1,555 B for deflate's default level; libzstd level 3 gave 1,696 B and needs a C build. Deflate stays, through `flate2`, as today.
- **`tesserae save dump <world-dir> [--sequence N]`** prints the newest (or named) Commit as JSON on stdout: the manifest, each Chunk's palette by name and its indices as runs, each paused share. Read-only, takes no lock, parsed beside `validate` and `package`, shipped in normal builds.

## Testing Decisions

- Tests exercise the Writer and recovery through the directory, as today's do, and the Server through the existing `Rig`.
- **Crash safety.** The error-injection test covers the new phases (pack body, pack sync, pack rename) and every phase keeps the previous Commit recoverable. The killed-subprocess test gains a kill between the pack's rename and the manifest's, after which the previous Commit reopens and the orphan pack is swept by the next writer.
- **Retention.** After any number of writes exactly three Commits remain; a failed delete does not fail the write; a corrupt newer Commit ages out; nothing is deleted before the third completion.
- **Garbage collection.** A pack a kept Commit names is never deleted; one only an aged-out Commit named is; compaction moves records and the old pack goes.
- **Incremental.** Editing one Chunk after a save writes a pack holding one Chunk record, whatever else the World holds; an unchanged paused share is not written again.
- **Rescans.** An unchanged wet Chunk evicted mid-rescan leaves without a Commit and no paused entry, and settles the same when it returns as one never evicted.
- **Compatibility.** Format 1 and 2 Commits open and their Chunks restore Voxel for Voxel; a manifest with an unknown format is refused as incompatible, never skipped as corrupt; a record whose hash does not match makes its Commit corrupt, so recovery falls back.
- **Dump.** Dumping a Commit with Chunks and paused shares prints JSON that parses and names every Chunk.
- Measurement, not a test: the same 3-minute flight before and after each ticket (below).

## Out of Scope

- Chunks stored as deltas from World generation ([ADR-0031](../../docs/adr/0031-a-saved-chunk-is-stored-whole-not-as-a-delta-from-generation.md)).
- Hourly or long-term retention, and any user-facing backup.
- SQLite or region files as the container; zstd.
- Shrinking the Map, entities or containers sections.
- Evicted changed Chunks still stay in memory, palette-compressed, as today.
- Windows runtime testing (unchanged from today).

## Further Notes

**Measurement.** One script and one World: a `dev,profiling` release build, `TESSERAE_GENERATOR=3`, a fresh `TESSERAE_WORLD_DIR` (default seed), the `flight` profiling fixture with the capture flying 180 s at 12 m/s after 20 s, offscreen. Recorded per run: disk used (`du`), Commits written, bytes written per Commit, save worker time per Commit from the profiling CSV, and the host's load. The table lands in ticket 03's Comments and the PR.

Baseline, d4179634, load average 8 to 10 (the user's own earlier run of the same kind wrote 99 Commits, 195 MB):

| Measure                     | Baseline                                                      |
| --------------------------- | ------------------------------------------------------------- |
| Disk used                   | 141.8 MiB                                                     |
| Commits written and kept    | 74                                                            |
| Bytes per Commit            | 2.0 MB median, 3.2 MB last                                    |
| Save worker time per Commit | 193 ms median, 397 ms max                                     |
| Newest Commit by section    | `paused` 2.2 MB (4,333 entries), `chunks` 0.7 MB (123 Chunks) |
