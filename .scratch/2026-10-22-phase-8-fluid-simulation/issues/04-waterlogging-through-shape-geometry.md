# 04: Waterlog partial Shapes without leaks

Status: ready-for-agent
Blocked by: 03

Spec: [decisions 3, 4, 5 and 11](../spec.md).

## What to build

Let water occupy and flow through accessible empty space around a solid
Shape, with clipped visible surfaces and unchanged solid collision/selection.

## Acceptance criteria

- [ ] Slabs, stairs, posts, fences, and declared nodeboxes can coexist with
  compatible water without changing solid Material/Shape/Orientation.
- [ ] Passage derives from the existing sixteenth-resolution Shape geometry
  at registration, including Orientation and actual derived connection arms.
- [ ] Tests prevent flow through sealed faces, jointly sealed interfaces,
  internal partitions, and disconnected voids. Sealed cavities do not become
  wet merely because another void in the same Voxel is wet.
- [ ] Only one fluid Material can occupy a Voxel, with region state where
  necessary to preserve reachability. Arbitrary non-solid Materials are not
  implicitly replaced by water.
- [ ] Partial solid placement preserves compatible fluid; full solid
  placement displaces it; solid removal preserves remaining fluid. Source
  versus flowing identity survives these operations.
- [ ] Source placement/removal targets waterlogged state without deleting its
  solid. Invalid or cap-exceeding operations remain atomic.
- [ ] A half-metre gravity drop within partial geometry contributes to the
  physical reset-drop rule at different Voxel sizes.
- [ ] Meshes clip water to accessible voids without changing collision or
  selection. Connection changes wake passage and remesh affected boundaries.
- [ ] Light increases/decreases and stale Snapshots remain correct under the
  explicitly shared per-Voxel light approximation. Precise per-void light is
  not silently added.
- [ ] Dry/full-cube paths retain their cheap gates; topology does not become
  a per-tick full 16-cubed simulation for every terrain Voxel.

## Verification and completion

Run relevant native geometry/meshing/light tests and controlled Server tests.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- The proposed waterlogging model preserves disconnected geometry for passage
  while retaining the existing light-storage resolution.
