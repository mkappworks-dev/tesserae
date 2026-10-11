# The incremental solver

Status: done
Blocked by: 02

What makes this step's title true: an edit re-floods the region it reaches, not
the Chunk it happened in. DESIGN.md's "one torch re-floods a small region, not
a chunk" rules out a per-Chunk re-solve by name.

## What to build

In `light.rs`:

- `pub trait LightVolume` with `voxel(&self, [i32; 3]) -> Option<Voxel>`,
  `light(&self, [i32; 3]) -> Light` and `set_light(&mut self, [i32; 3], Light)`.
  `None` from `voxel` means the volume does not extend there and the fill
  stops - it is not air. `[i32; 3]` rather than a vector type because core has
  no dependencies and is not acquiring `glam` for this.
- An increase pass: a queue of Voxels whose light just rose, pushing
  `transmitted(...)` into transparent neighbours that are dimmer.
- A decrease pass: a queue of `(Voxel, the level it had)`. A neighbour at or
  below `transmitted(level, face)` was lit from here, so clear it and recurse;
  a neighbour above it has an independent source at least as strong, so queue
  it for the increase pass that runs afterwards.
- `pub fn relight(volume: &mut impl LightVolume, edited: [i32; 3])` - the entry
  point an edit calls. It reads the Voxel now at `edited`, seeds a decrease from
  the light that was there and an increase from the new emission and the
  surviving neighbours, then runs both passes.

## The sky-light trap in the decrease pass

Sky light at full strength transmits downwards undimmed, so a naive
`neighbour < level` test never fires for the Voxel below and the column under a
newly placed roof stays lit. Comparing against `transmitted(level, face)`
rather than against `level` is what fixes it, and it is the reason
`transmitted` is one shared function rather than two inline expressions.

## Why this is not the bulk solver run again

Spec decision 6. A from-scratch per-Chunk re-solve is Gauss-Seidel over a max
system: it converges upwards but not downwards, so two Chunks each holding
light up at the other's level is a stable fixed point with no source behind it.
The decrease pass is sound only because it tracks provenance.

## Tests

- Placing an emitter lights its neighbourhood, and removing it returns every
  Voxel to its previous value - exactly, not approximately.
- Placing a roof over an open column clears the sky light below it and then
  refills it from the sides at 14, 13, ... The test that fails without the
  `transmitted` comparison above.
- The two solvers agree: build a scenario, solve it with `light_chunk`, build
  the same scenario incrementally one edit at a time, and assert the results
  are identical. This is the guard against the two paths drifting.
- An edit 15 Voxels from a Chunk boundary touches the neighbour; one 16 Voxels
  away does not.
- A `None` region bounds the fill rather than being treated as air.

## Comments

`ChunkLight`'s derived `PartialEq` compared how light was *stored*, not the
light. A Chunk edited and then restored holds a dense array of exactly what a
uniform one holds, because `set` promotes and never demotes — so the Server's
"did this re-solve change anything?" test would have fired on a Chunk whose
light had not moved, and re-meshed it for nothing. Equality is now written out
by hand and compares values across the two representations.
