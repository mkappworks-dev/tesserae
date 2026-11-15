# 03: The Server captures the Player's Map

Status: ready-for-agent

Blocked by: 01

Spec: [Phase 19](../spec.md).

## What to build

A new Bevy-free Map module in the core crate holds the Map: 2 m samples (surface
height, top Material id, Biome id as of when seen), the Map tile (a fixed square
of samples in metres), the Explored test and a per-Player Map. A new Server
system, run each fixed step after the Player's body is advanced, scans the
columns within 96 m of the Eye whose Chunks are held and lit (the same "held and
not unlit" test the Soundscape and Plants systems use) and records them. It never
marks a column whose Chunk is unloaded, scans a bounded number of samples a step
from a cursor so the tick never stalls, reads the held level-0 Chunks so a placed
Voxel or a dug lake is what is recorded, and takes the Biome from the generator's
pure column query. Everything is in metres (ADR-0003).

Nothing is visible yet; this is verified by tests at the Server seam and the
core unit seam.

## Acceptance criteria

- [ ] The same World at Voxel size 1.0 and 0.25 yields the same Map samples.
- [ ] Unloaded or unlit terrain is never marked Explored.
- [ ] A Voxel the Player placed or removed shows in the sample on the next visit; an earlier sample is replaced.
- [ ] The scan is bounded per Logical step and a cursor resumes it; a test shows the bound holds.
- [ ] The Map module has no Bevy or wgpu dependency (the core CI check passes).
- [ ] Server tests mimic the Soundscape tests (hand-built Chunks, a Player at a known position); fmt, clippy and tests pass.
