# 05: The small caches, each if its symbol is worth it

Status: ready-for-agent

Blocked by: 02, 03, 04

Spec: [Generation cost](../spec.md).

## What to build

First the gate: a sampled `flight` after 04. Each of these is built, one commit each, only if its symbol is more than 3% of generation at some level; otherwise closed in Comments with its figure:
- the cavern field reads a noise corner cache (`fbm3_cached`), as the tunnel fields do;
- `sealed_below_m` and the smoothing's `filled` read the column cache where their points fall on its lattice, and drop their duplicate `water_level_m` call;
- one `claims_near` at the widest margin serves a Chunk, filtered for each use;
- an ore remembers its last vein cell and its draws while a column is walked down.

## Acceptance criteria

- [ ] The gate's figures in Comments.
- [ ] For each built: the fingerprint and the committed digests match, and a test of its own seam (the cached cavern field equals `fbm3`; the seal and `filled` equal their uncached answers; the ore equals today's per Voxel).
- [ ] For each built: bench figures in Comments.
