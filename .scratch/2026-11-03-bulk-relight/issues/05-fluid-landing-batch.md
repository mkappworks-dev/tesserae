# 05: Fluid landing on a batch

Status: ready-for-agent
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
- [ ] A test drains a line of lava sources outdoors and checks the light after
      settling equals a fresh solve.
- [ ] The core bench has a case removing a row of emitters under open sky, one
      at a time and as one batch, and the figures are in Comments.

## Comments
