# 03: Binary records and `save dump`

Status: ready-for-agent

Blocked by: 02

Spec: [Save: retention, incremental commits and binary Chunk records](../spec.md).

## What to build

Pack records become binary: a kind and version, then a deflated body of a record-local name table and the content. A Chunk record is its palette by name-table index and one index per Voxel, bit-packed or as byte planes, whichever measures smaller on the baseline World's Chunks. A paused share record is its timers, wakes, rescans and landing cursors with Materials through the name table. `tesserae save dump <world-dir> [--sequence N]` prints a Commit as JSON.

## Acceptance criteria

- [ ] Chunks and paused shares round-trip exactly through binary records.
- [ ] Byte planes against bit-packing, measured on real Chunks, recorded below; the smaller is used.
- [ ] A truncated or overlong record body is corrupt, not a panic.
- [ ] `tesserae save dump` prints JSON that parses and names every Chunk and share; it takes no lock and changes no file.
- [ ] `docs/world-persistence.md` describes the record layout and the dump command.
- [ ] The before and after table for all three tickets is in the Comments below and in the PR.
