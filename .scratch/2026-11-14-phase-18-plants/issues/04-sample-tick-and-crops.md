# 04: Sample tick and crops

Status: ready-for-agent

Blocked by: 02, 03

Spec: [Phase 18](../spec.md).

## What to build

A crop placed in a held Chunk ripens over its declared seconds. The Sample
kernel in `tesserae-core` draws positions and rolls from a hash of the seed,
Chunk, step and index; the Server runs it each Logical step for held level-0
Chunks whose palette holds a Plant Material, checks soil, light, Climate and
water against the Snapshot, and lands the next Stage through `set_occupants`
under `PLANTS_LANDED_PER_STEP`. A failing condition holds the Stage. The bench
fixes the constants.

## Acceptance criteria

- [ ] A crop advances Stage by Stage and holds when soil, light, Climate or
      water fails, in a Server headless test, at Voxel sizes 1.0, 0.5 and 0.25.
- [ ] Equal seed, state and events give equal Stages at different worker
      counts.
- [ ] An unheld Chunk, and one whose light solve is pending, is skipped, never
      read as Air; a Chunk held again resumes without credit for time away.
- [ ] Growth past the landing bound lands the next step; a read that changed
      drops the change and Samples again.
- [ ] A Chunk with no Plant Material costs nothing measurable; the bench
      numbers are recorded in the PR and in DESIGN.md's step 18 notes.
- [ ] No new save record: a reopened World continues from the Stage Material.

## Comments

Left `ready-for-agent`: the bench numbers (and the DESIGN.md step 18 note) are
not done, so `PLANTS_LANDED_PER_STEP = 64` is a starting value, not a measured
one. The "no new save record" criterion holds by construction (nothing is
stored) but has no reopen test yet.

- The Sample tick runs inline on the tick, at the end of each Logical step
  (after the step's fluid landings, before `next_step`), in `FluidSim`'s clock
  and through the same `edit::Batch::set_occupants`. It reads the live store
  rather than a worker Snapshot, so nothing it reads can be stale within the
  step; only a change deferred past the bound is read again next step (roll
  kept, dropped if its Voxel or needs changed).
- Reads go only to Chunks the simulation has admitted; an unadmitted Chunk or
  one with a light solve pending is skipped, and a read into one fails the
  need, so the Voxel holds.
- Climate is read at the Voxel's corner in metres, as generation reads Biomes.
