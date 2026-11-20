# 02: One drainage ask answers a column's cut, Water level and Biome

Status: ready-for-agent

Blocked by: 01

Spec: [Generation cost](../spec.md).

## What to build

`Drainage::wet`'s Water level does not read the ground height, so one call at the base height answers both the valley cut and the Water level of a raw column, in `build_columns` (four-wide) and in `contained_at_m`. The column's Biome, the sea-floor rock and the overhang reach are decided from the fields, base height and cut the raw pass computed, rather than `site` and `seabed_rock` asking them again.

## Acceptance criteria

- [ ] Version 3 matches the fingerprint at 1.0 and 0.5 m, the crop-field grid included; the committed version 3 digests pass unchanged.
- [ ] A test over many seeds and points: the fused raw column and Biome equal the separate asks bit for bit, four-wide and scalar.
- [ ] A Chunk after `forget_caches` equals one generated warm.
- [ ] Bench figures against 01's binary in Comments, with `uptime`.
