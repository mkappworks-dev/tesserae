# Bench a Chunk a cave crosses, in all three stages

Status: ready-for-agent
Blocked by: 02

Without it the budget is being sized against terrain that no longer exists.

## What to build

- A fixture finder in `benches/meshing.rs` that scans for a deep Chunk inside
  the cave band that is not uniform, deterministically (fixed seed, fixed scan
  order), plus a deep Chunk inside the band that is.
- `worldgen/cave_crossed` (the carved path), `worldgen/cave_bound_declines` (the
  bound ruling a deep Chunk out, which is the new cost of the old early-out),
  `worldgen/cave_bound_passes_empty` (it could carve and does not).
- `light_chunk/cave_dark_neighbours` (what a Chunk arriving costs: unsolved
  neighbours) and `light_chunk/cave_mouth` (with sky arriving through a mouth).
- `mesh_chunk/cave_crossed`.
- The existing benches keep their names and fixtures, so the before/after
  comparison in the PR is like for like.

## Acceptance criteria

- The before/after table is in Comments, taken in isolation (not in a full run
  disturbed by other work), and says which rows are new.
- The surface-crossing rows do not regress beyond noise, or the regression is
  explained: `straddling` now pays for the Chunk-level bound.
- `cargo bench --no-run` builds without `--features dev`.

## Comments
