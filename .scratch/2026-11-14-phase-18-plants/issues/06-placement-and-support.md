# 06: Placement rules and support

Status: ready-for-agent

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

Planting on the wrong ground is refused with a reason, and a plant whose support
goes breaks. The `soil` rule is Edit validation for Place and for a Layout in
both modes, with a refusal naming the need; light, Climate and water are never
placement rules. When a plant Voxel's soil, attachment or standing fluid
changes, the Voxel is woken through the existing wake-on-change and re-reads its
support, breaking and dropping its Seed in the same Logical step. Nothing is
polled. Mod Commands and burning reach plant Voxels through the same write
path.

## Acceptance criteria

- [ ] A Seed on non-soil is refused with a reason; a Layout is refused whole.
- [ ] A Seed placed in the dark is accepted and waits.
- [ ] Removing, replacing or burning the soil breaks the plant in that step and
      drops its Seed; a plant on a Mod Command's new soil is unaffected.
- [ ] No per-tick scan of plants appears in a profile of an idle World.
