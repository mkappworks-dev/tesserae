# 03: Binary records and `save dump`

Status: done

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
- [x] The before and after table for all three tickets is in the Comments below and in the PR.

## Comments

**Byte planes against bit-packing**, measured on the baseline World (`d4179634`, the 3-minute flight) by recapturing its Chunks' indices and deflating each way at the default level:

| Chunks                                  | Format 2 (planes, deflate-fast) | Byte planes, default | Bit-packed, default | Bit-packing smaller for |
| --------------------------------------- | ------------------------------- | -------------------- | ------------------- | ----------------------- |
| Newest Commit's 123                     | 302,565 B                       | 154,505 B            | 166,935 B           | 36                      |
| 315 distinct versions across 74 Commits | 847,832 B                       | 434,286 B            | 470,873 B           | 77                      |

Byte planes are kept: the low plane, the high one only past 256 cells, neither for a single cell. Median Chunk: 1,404 B as planes, 1,452 B bit-packed, palette of 28.

**The baseline's newest Commit rewritten as format 3**, offline: a 412 KB pack (123 Chunks and 4,333 paused shares, binary) against the 3.2 MB format 2 Commit, and a 611 KB manifest, most of it the 4,333 shares' references; with rescan-only shares no longer paused, the flight should write few of them. Every Chunk restored Voxel for Voxel.

**Before and after**, the 3-minute flight (fresh version 3 World, default seed, `flight` fixture, 180 s at 12 m/s, `dev,profiling` release build) on 2026-11-18, rebased on main `b88e7c12`, load 4.8 to 7.9. Main and ticket 01 ran twice (both runs shown); tickets 02 and 03 once each after the paused-rescan fix in ticket 02's Comments. Bytes written are every `.commit` and `.pack` the run wrote, polled while it ran, including those retention deleted.

| Measure                          | main                  | 01 retention          | 02 incremental | 03 binary records |
| -------------------------------- | --------------------- | --------------------- | -------------- | ----------------- |
| Disk used                        | 265.8 / 266.3 MiB     | 16.9 / 16.9 MiB       | 4.5 MiB        | 3.0 MiB           |
| Files kept                       | 73 Commits            | 3 Commits             | 3 + 19 packs   | 3 + 18 packs      |
| Commits written                  | 73 / 73               | 73 / 73               | 73             | 73                |
| Bytes written per Commit, median | 4.2 / 4.3 MB          | 4.2 / 4.3 MB          | 524 KB         | 457 KB            |
| Bytes written per Commit, max    | 6.0 / 6.0 MB          | 6.0 / 6.0 MB          | 990 KB         | 722 KB            |
| Bytes written in the flight      | 279 / 279 MB          | 280 / 280 MB          | 39.0 MB        | 33.5 MB           |
| Save worker per Commit, median   | 599 / 590 ms          | 623 / 589 ms          | 93 ms          | 97 ms             |
| Save worker per Commit, max      | 1,028 / 970 ms        | 1,020 / 1,087 ms      | 256 ms         | 264 ms            |

Ticket 03 halves the packs (12.5 MB to 7.2 MB over the flight, 166 KB to 92 KB per Commit median); its save time is within the runs' noise of ticket 02's. What is left is the manifest, 365 KB per Commit, written whole each time: `simulation` 94 KB (the active share, inline), `parameters` 87 KB, `map` 66 KB, `chunks` 58 KB (736 references), `paused` 34 KB (463), `entities` 25 KB. The spec leaves those sections as they are.
