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
