# 03: Binary records and `save dump`

Status: ready-for-agent

Blocked by: 02

Spec: [Save: retention, incremental commits and binary Chunk records](../spec.md).

## What to build

Pack records become binary: a kind and version, then a deflated body of a record-local name table and the content. A Chunk record is its palette by name-table index and one index per Voxel, bit-packed or as byte planes, whichever measures smaller on the baseline World's Chunks. A paused share record is its timers, wakes, rescans and landing cursors with Materials through the name table. `tesserae save dump <world-dir> [--sequence N]` prints a Commit as JSON.

## Acceptance criteria

- [x] Chunks and paused shares round-trip exactly through binary records.
- [x] Byte planes against bit-packing, measured on real Chunks, recorded below; the smaller is used.
- [x] A truncated or overlong record body is corrupt, not a panic.
- [x] `tesserae save dump` prints JSON that parses and names every Chunk and share; it takes no lock and changes no file.
- [x] `docs/world-persistence.md` describes the record layout and the dump command.
- [ ] The before and after table for all three tickets is in the Comments below and in the PR.

## Comments

**Byte planes against bit-packing**, measured on the baseline World (`d4179634`, the 3-minute flight) by recapturing its Chunks' indices and deflating each way at the default level:

| Chunks                                  | Format 2 (planes, deflate-fast) | Byte planes, default | Bit-packed, default | Bit-packing smaller for |
| --------------------------------------- | ------------------------------- | -------------------- | ------------------- | ----------------------- |
| Newest Commit's 123                     | 302,565 B                       | 154,505 B            | 166,935 B           | 36                      |
| 315 distinct versions across 74 Commits | 847,832 B                       | 434,286 B            | 470,873 B           | 77                      |

Byte planes are kept: the low plane, the high one only past 256 cells, neither for a single cell. Median Chunk: 1,404 B as planes, 1,452 B bit-packed, palette of 28.

**The baseline's newest Commit rewritten as format 3**, offline: a 412 KB pack (123 Chunks and 4,333 paused shares, binary) against the 3.2 MB format 2 Commit, and a 611 KB manifest, most of it the 4,333 shares' references; with rescan-only shares no longer paused, the flight should write few of them. Every Chunk restored Voxel for Voxel. This is not the flight measurement, which waits for step 17 to merge and a quiet host.
