# 01: Crop fields in the bench, a warm-up that solves every region, and the starting figures

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Generation cost](../spec.md).

## What to build

The version 3 bench gains a World with crop fields like Terra's (one entry near the lake with a ditch on a 64 m lattice, one anywhere on a 96 m lattice) and cases over it: a straddling Chunk on a held field and a column of Chunks over one. Its warm-up solves every drainage region the case's Chunks and their neighbours within a tree's reach touch, so a lone Chunk's figure is generation, not a region solve. The uncommitted fingerprint gains a grid over the crop-field World, its baseline taken at this commit. The starting figures are recorded: the bench at this commit, the flight figures already in the spec's Further notes, and a sampled `flight` with the top symbols per level.

## Acceptance criteria

- [ ] The bench runs the crop-field cases, and the lone river and cavern cases no longer include a region solve.
- [ ] The starting bench figures and the sampled flight's top symbols, with `uptime` and how they were taken, are in Comments.
- [ ] No generator code changes; the committed digests pass and the fingerprint's existing lines match 4e78a50f's.
