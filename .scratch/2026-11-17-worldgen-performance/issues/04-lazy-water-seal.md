# 04: The seal beside water is computed once per column

Status: ready-for-agent

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

The height below which rock beside water stays solid is computed the first time a carve asks for a column and kept in the column cache, one atomic per column, so the Chunks over a Cell and the coarse levels stop asking nine Water levels per carved column each time. Two workers that race compute the same value.

## Acceptance criteria

- [ ] Version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [ ] Generated water has no air below or beside it, and flooded caverns stay sealed from dry tunnels, at 1.0 and 0.5 m.
- [ ] Bench figures before and after in Comments.
