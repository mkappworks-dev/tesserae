# 08: Keep fluid and fire changes visible through LOD

Status: ready-for-agent
Blocked by: 06, 07

Spec: [decisions 7 and 11](../spec.md).

## What to build

Keep full-detail and held level-1 views responsive to flowing, waterlogging,
reactions, burning, and extinguishing, without moving dynamic water to the
generated waterline or simulating distant terrain.

## Acceptance criteria

- [ ] Auxiliary fluid/burning changes invalidate level-1 covers even when
  the packed solid Voxel is unchanged; eviction removes held changes under
  ADR-0011's existing semantics.
- [ ] Level 1 derives a documented volume-based approximation of held changed
  state. Coarse levels never simulate fluids, exposure, or fire.
- [ ] Higher levels retain generator-only content and coarse-light limits;
  document the resulting distance discontinuities.
- [ ] Generated-waterline corrections keep their existing lake seam behavior
  where applicable and never relocate arbitrary waterfall/waterlogged surfaces.
- [ ] Ring masking continues to avoid double coverage or holes for both
  generated and dynamically changed water at supported Voxel sizes.
- [ ] Ongoing changes do not starve full-detail or level-1 mesh delivery.
  Coalesce invalidation and retain follow-up work after an older mesh lands.
- [ ] Light Snapshot validity includes fluid, fire, and geometry changes;
  both brightening and darkening agree with a fresh solve under the explicitly
  shared per-Voxel light contract.
- [ ] Held/scrambled Jobs and controlled-tick integration tests verify stale
  result handling, boundary changes, mesh redelivery, and eventual settling.

## Verification and completion

Run relevant LOD, meshing, light, and Server integration tests and comparisons.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Per-void light storage and a separate coarse emitter flood are not added.
  This ticket integrates the declared approximation, rather than expanding
  thermal/fluid simulation to distant LOD rings.
