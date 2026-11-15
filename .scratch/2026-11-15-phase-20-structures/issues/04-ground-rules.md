# 04: Structures stand only on suitable ground

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

A lattice draw is accepted only where every sampled footprint column, a metre
apart, is in a Biome that lists the Template, the ground varies by at most the
Template's tolerance, no lake covers a column and no cave cuts a column's
surface. Up to 36 draws are tried per lattice cell and the first that passes
wins. The Template's Ground layer sits at the footprint's median ground height,
layers below it overwrite underground, and the foundation Material fills any
gap beneath the lowest layer down to ground.

## Acceptance criteria

- [ ] No accepted Structure stands in a lake, over a cave mouth, on ground
      steeper than its tolerance or in a Biome that does not list it.
- [ ] A Template with a cellar has its cellar underground and its Ground layer
      level with the surface.
- [ ] A foundation Material fills the gap on uneven ground and is absent when
      none is declared.
- [ ] Over caves and smoothed ground, a Structure is still whole across Chunk
      borders and equal at all three Voxel sizes.
- [ ] A rejected draw leaves the cell empty or takes a later attempt, the same
      whichever Chunk is generated first.
