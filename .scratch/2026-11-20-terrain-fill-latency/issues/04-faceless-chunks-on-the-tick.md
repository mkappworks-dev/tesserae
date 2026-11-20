# 04: A Chunk that cannot have a face is sent its empty mesh on the tick

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

A level-0 Chunk that cannot have a face whatever the light (the cases `light_does_not_revive_either_declined_case` covers) is sent its empty mesh on the tick it lands, without a mesh job or the settled-light wait.

## Acceptance criteria

- [ ] A streaming test: such a Chunk's empty mesh lands on its tick, and every other Chunk still waits for settled light.
- [ ] The holes instrument reports no more fill or rule holes than 01's figures: an empty mesh sent early marks its Cell drawn and may discard a coarser level's ground poking into it before the Chunk below arrives.
- [ ] `flight`, `sprint-flight` and `teleport` against 01's figures, three interleaved rounds, in Comments with `uptime`.
