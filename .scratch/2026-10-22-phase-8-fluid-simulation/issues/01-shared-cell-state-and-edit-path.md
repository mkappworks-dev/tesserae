# 01: Preserve cell state through one Server Edit path

Status: ready-for-agent
Blocked by: None

Spec: [decisions 2, 3, 5 and 11](../spec.md).

## What to build

Prefactor authoritative cell state so the packed solid Voxel can coexist
with fluid and combustion data without changing today's dry terrain and
generated-water behavior. Keep one canonical interpretation and one
Server-private World-change path for all callers.

This is the explicit prefactoring exception to vertical slices: it prepares
the shared storage/Edit interface before simulation starts changing cells.

## Acceptance criteria

- [ ] Packed Voxel layout and permanent Material/Shape ids are unchanged.
  No Orientation or Shape bits encode fluid or burning state.
- [ ] Owned native cell/Chunk state and immutable Snapshots carry the
  additional state without independently mutable copies of the same water.
- [ ] Dry/unburning Chunks allocate no dense auxiliary arrays; uniform lake
  water retains compact storage. Record memory evidence in Comments.
- [ ] Existing generation, player Edits, Mod Commands, and dev builders use
  the shared mutation rules. Validation, shaped caps, light, boundary meshes,
  and level-1 invalidation still work.
- [ ] Auxiliary-state changes invalidate relevant Snapshot dependencies even
  when the solid Voxel is unchanged. Old Chunk lifetimes cannot be confused
  with new ones at the same coordinate.
- [ ] Existing generated-water appearance, solid picking, collision, and
  0.1 Mod behavior remain covered through native and Server integration tests.
- [ ] Capture pre-change native hot-path baselines for affected meshing,
  collision, and LOD comparisons, following the spec's measurement policy.
- [ ] No Client mutator, core Bevy/wgpu edge, or dependency security downgrade
  is introduced.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- 2026-10-22: triaged ready-for-agent after the breakdown review. The only
  ticket with no blocker, so implementation starts here.
