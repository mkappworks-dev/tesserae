# Size the caps and the thread count from measurement

Status: ready-for-agent
Blocked by: 05

Every number so far is a placeholder. Spec decision 6 and "What this assumes
about per-Chunk cost".

## What to build

- Measure on the ticket 01 harness, five runs per configuration: async compute
  threads at 2, 3 and 4 (set through `TaskPoolPlugin` options in `main.rs` if
  one wins); in-flight caps over a sweep per stage; landing caps over a sweep
  for mesh.
- Measure the Client's cost per delivered mesh (`chunk_mesh_to_mesh` plus
  `Assets<Mesh>::add`), which is what the mesh landing cap actually bounds. If a
  full landing cap of meshes in one frame costs more than a couple of
  milliseconds, say so: it is the next thing to move off the render thread, and
  a loose end the spec already names.
- Set the defaults. Write `StreamingBudget`'s doc comment in the same form the
  old one had: what was measured, on what, what it implies, and what the number
  does *not* bound. State what a bad value costs (fill latency or stale
  priority, not frame time) and what the sizing assumes about per-Chunk cost,
  pointing at the spec's four assumptions.
- Update DESIGN.md's "Per-ring LOD chunk budget" open item: the three budgets
  are no longer sized against each other, and what it measures has changed
  meaning.

## Acceptance criteria

- The sweep tables are in Comments, with the machine and load.
- The doc comment contains no number whose reason is gone (the old "relight is
  half of generation" paragraph is rewritten or removed).
- The staleness window (`cap x mean cost / threads`) for the chosen defaults is
  computed from measured costs and stated.
- `./scripts/cargo-ci` passes.
