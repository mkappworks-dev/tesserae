# 03: Worldgen stands a Structure

Status: ready-for-agent

Blocked by: 02

Spec: [Phase 20](../spec.md).

## What to build

A Biome lists `structures`: a Template, a spacing in metres and a chance, at
most 4 rows, with spacing no smaller than twice the Template's longer side.
Worldgen holds a Structure per lattice cell by a salted hash of the seed and
the cell, with its rotation, inset so two never touch, and writes its cells
into the Chunks it reaches: every non-Keep cell, Air included, overwrites what
is there. The pass exists only when a Biome lists a Structure. This ticket
places on any ground; the ground rules come in ticket 04.

## Acceptance criteria

- [ ] A Structure is whole across Chunk borders, equal in metres at 1.0, 0.5 and
      0.25 m, and the same whichever Chunk is generated first, with more than a
      few crossing a border.
- [ ] All four turns appear and each Structure's footprint is the same in metres
      turned.
- [ ] Keep cells leave the terrain untouched and Air cells carve it.
- [ ] A World whose Biomes list no Structure generates the digests recorded
      before this ticket, and a version 1 World is unchanged.
- [ ] A fifth row, an unknown Template or a spacing under its floor is a load
      error naming the Biome.
- [ ] A Biome stays inline within its size limit with 4 rows.
