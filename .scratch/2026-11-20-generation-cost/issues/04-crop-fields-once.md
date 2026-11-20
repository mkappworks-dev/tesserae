# 04: Each crop-field Lattice cell is decided once

Status: ready-for-agent

Blocked by: 01

Spec: [Generation cost](../spec.md).

## What to build

Whether a Lattice cell of a Biome's crop-field entry holds a field, and the first drawn place the ground takes, is decided once per `(Biome, entry, cell)` and kept in a bounded map like `CellMap`: a read-write lock, the oldest half dropped at the bound. `fields_near`, and so every Chunk, `cell_plants` and `plants_clear_m`, read it, including the test that a later entry gives way to an earlier one it crosses. `field_accepts` is unchanged. `forget_caches` clears it.

## Acceptance criteria

- [ ] Version 3 matches the fingerprint at 1.0 and 0.5 m, the crop-field grid included; the committed version 3 digests pass unchanged.
- [ ] A field crossing a Chunk border is the same from both Chunks, cold or warm, in any order; a Chunk after `forget_caches` equals one generated warm.
- [ ] Reaching the bound keeps the newer half.
- [ ] Bench figures against 01's binary in Comments, with `uptime`.
- [ ] The mid-point flight figures and sampled symbols in Comments.
