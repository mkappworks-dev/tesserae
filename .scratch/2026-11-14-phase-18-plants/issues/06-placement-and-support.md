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

## Comments

Left `ready-for-agent`: burning the soil is not tested (it lands through the
same `Batch::set_occupants`, so the same wake applies), and "no per-tick scan in
a profile of an idle World" holds by construction and by a test that an
untouched unsupported plant stays, but no profile was taken.

- Support is read from `store.take_changed()` in `absorb_changes` and
  `wake_landed` (`fluids.rs`), through `fluids/plants.rs::break_unsupported`:
  each changed cell and its six neighbours, so a World with no change reads
  nothing. A broken plant is one more change, so a chain settles within
  `SUPPORT_PASSES`.
- The rule is `PlantTable::lacks_soil`: soil in the Voxel below (above for a
  downward column, which may also stand on its own Material across that face);
  a climber needs soil in any one of its six neighbours. Standing in a fluid is
  not a placement or support rule: the row has no field to tell kelp from cane,
  whose `water` is a growth reach, so a column stands on soil like the rest.
- The refusal is `RejectReason::NeedsSoil` plus a `PlantNeedsSoil { text }`
  message ("Wheat needs soil."), as `HistoryNeedsItems` rides beside
  `ItemNeeded`; the soil is named by its Item.

