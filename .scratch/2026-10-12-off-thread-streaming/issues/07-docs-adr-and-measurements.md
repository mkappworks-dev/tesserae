# Docs, ADR, measurements

Status: ready-for-agent
Blocked by: 01, 02, 03, 04, 05, 06

## What to build

- `CONTEXT.md`: Job, Snapshot, Landing, under Runtime.
- `docs/adr/0007-streaming-jobs-run-on-bevy-task-pools.md`: Jobs run on Bevy's
  task pools from `tesserae`, never in `tesserae-core`; the store holds `Arc`s
  and a Job receives only shared references. The two things expensive to change
  once other code leans on them.
- `docs/architecture.md`: the streaming diagram and the `streaming.rs` line,
  which still says "per-tick budget".
- DESIGN.md: the crate-layout line promising `rayon` in core (see spec decision
  2) and the step 3 and step 4 notes that listed this as a deferred move.
- `streaming.rs`'s module doc: add the in-flight case to the extent-ordering
  argument (spec decision 8).
- The PR description, per the template, printed in full in the session as well
  as attached. It must state what replaced the deterministic per-tick count and
  what was given up for it, and the cost assumptions the caves work should
  check.
- Before/after tables from ticket 01's harness, five runs each, median and
  range.
- A `dev` screenshot taken while flying.

## Acceptance criteria

- `npm run fmt` and `cargo fmt --all` clean; `./scripts/cargo-ci` passes.
- The PR body fills in the template, strikes through the inapplicable items, and
  does not end with the generated-by line.
