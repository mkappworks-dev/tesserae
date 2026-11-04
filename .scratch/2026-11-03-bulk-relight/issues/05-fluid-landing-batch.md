# 05: Fluid landing on a batch

Status: done
Blocked by: 03

Spec: [bulk relight](../spec.md), decision 6.

## What to build

The landing loop in `server/fluids.rs` (`set_occupants` per chosen proposal)
writes into one `edit::Batch` per step and finishes it once, before the step's
journal is taken.

- `still_valid` keeps reading the store after each landing, which a batch
  allows: it reads Occupants, not light.
- `LANDED_PER_STEP` and `EMITTERS_PER_STEP` are unchanged.

## Acceptance criteria

- [ ] Every fluid and fire test passes unchanged, including the determinism
      tests: equal state and events give equal state.
- [x] A test drains a line of lava sources outdoors and checks the light after
      settling equals a fresh solve.
- [x] The core bench has a case removing a row of emitters under open sky, one
      at a time and as one batch, and the figures are in Comments.

## Comments

### Result, 2026-11-04

A fluid step lands as one `edit::Batch` relit once; `LANDED_PER_STEP` and
`EMITTERS_PER_STEP` are unchanged. All fluid and fire tests pass, including the
determinism ones, and `lava_draining_relights_in_batches_as_a_fresh_solve_would`
pins the light of three lava sources spreading and draining to a fresh solve.

Core bench, a row of 16 lanterns on a floor under open sky removed (release,
median of 10, load 3-5; batches 64 and 4096 are both one batch of 16 here, so
their gap is noise): one at a time 7.7 ms, one batch 2.3-3.0 ms, about 3x.

**Not shown: a faster lava drain at 0.25 m.** The `lava` scenario of the phase 8
harness (`measure_fluid_and_fire_scenarios`, debug profile, load 7-15, 9,665
ticks, unsettled in every run) gave fluid tick CPU p90 13.5 and 9.1 ms (base,
two runs) against 11.8 ms (new), max 20.2 and 18.2 against 19.1. Base run to
base run varies as much as base to new, and the runs did not reach the same step
count, so no difference is established. The step's cost there is evidently not
mostly relight, so the emitter bound stays 4 and ADR-0014 keeps raising it as a
separate decision. The third round of each side was killed by the background
time limit.
