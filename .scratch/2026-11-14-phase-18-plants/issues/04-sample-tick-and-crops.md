# 04: Sample tick and crops

Status: done

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

- [x] A crop advances Stage by Stage and holds when soil, light, Climate or
      water fails, in a Server headless test, at Voxel sizes 1.0, 0.5 and 0.25.
- [x] Equal seed, state and events give equal Stages at different worker
      counts.
- [x] An unheld Chunk, and one whose light solve is pending, is skipped, never
      read as Air; a Chunk held again resumes without credit for time away.
- [x] Growth past the landing bound lands the next step; a read that changed
      drops the change and Samples again.
- [x] A Chunk with no Plant Material costs nothing measurable; the bench
      numbers are recorded in the PR and in DESIGN.md's step 18 notes.
- [x] No new save record: a reopened World continues from the Stage Material.

## Comments

Measured by the benchmark pass (Apple M5, thread CPU time, interleaved rounds,
load average 8 to 9; rig in `fluids/plants/tests/bench.rs`, run with
`--ignored`). Per Logical step over 64 held Chunks: no plant table 0.11 us; a
plant table with no plant Material in any Chunk 0.62 us (0.01 us a Chunk); a
crop meadow of 1024 crops a Chunk at a 300 s mean 36 us (0.56 us a Chunk, 3.4
landings a step); the same at the 16 s floor with every Sample landing, 288 us
at the 64-landing bound (4.5 us a landing); 16 grown trees a Chunk 22 us; a
Chunk of nothing but crops with no soil, the most reads a Sample tick can make,
61 us (0.95 us a Chunk). `PLANT_SAMPLES` stays 32 and `PLANTS_LANDED_PER_STEP`
stays 64: a World of 1000 plant Chunks pays under 1 ms a step, and the bound at
saturation is under 2% of a 15.6 ms step. A save reopen test
(`a_grown_crop_and_tree_reopen_as_the_materials_they_grew_to`) covers the "no
new save record" criterion.

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
