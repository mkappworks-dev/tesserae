# 03: Binary greedy pass 1

Status: ready-for-agent
Blocked by: 02

Spec: [Packed quads](../spec.md).

## What to build

Item 3. Pass 1 builds each slice's face mask a row at a time with bit
operations and merges runs by trailing-zero scans, instead of a per-Voxel
`Option<FaceKey>` mask.

- The old pass 1 stays as a `#[cfg(test)]` reference.
- Records out are identical, in the same order.

## Acceptance criteria

- [ ] A test asserts identical records against the reference on the bench
      fixtures and on seeded random Chunks, translucent and wet included.
- [ ] Gated by ticket 06: `greedy_mesh` and `mesh_chunk` benches no slower,
      mesh worker time no worse.

## Comments

2026-11-17: written without measuring. Pass 1 keeps its merge and its order;
each row is a 32-bit mask of full cubes, a cube against a cube in an opaque
Chunk is dropped by a bit operation, and only faces that may show ask the
per-Voxel question. `pass1_reference` is the old pass, test-only;
`the_bitmask_pass_1_makes_the_reference_s_quads_in_its_order` compares the two
on terrain under three lights, buried terrain, solid and checkerboard Chunks
and eight seeded Chunks, opaque, translucent and wet, alone and among others.
Breaking the merge on purpose fails it. The gate is ticket 06.

2026-11-19: gated in ticket 06 against 02: level-0 mesh worker time -25-30%,
every meshing bench faster (`greedy_mesh/terrain` -76%), frame time no worse.
Passes; left open with 02, which it builds on.
