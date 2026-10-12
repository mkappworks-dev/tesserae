# Baselines, and the sweep and ray benchmarks they never had

Status: ready-for-agent

Spec decisions 7-9 change `collide.rs`, which has no benchmark, and decision 6
asks that pass 1 not move, which needs a number to not move from. This ticket
writes no feature code.

## What to build

- On `main` at or after f232c4e (the step-4/5 numbers quoted in older briefs
  are stale, so take them fresh), run `cargo bench -p tesserae-core --
--save-baseline main` and keep the output. Criterion baselines live in
  `target/` and are not committed; the table of medians goes in a `## Comments`
  section of this ticket so the numbers survive the machine.
- Add benches to `crates/tesserae-core/benches/meshing.rs` (or a new `collide`
  bench file registered in `Cargo.toml`) **on the unchanged code**, then save
  them in the same baseline: a fall onto a floor of cubes, a walk along a wall,
  a ray across terrain, and the same three on a floor of slabs. Same scenes
  after the change; they are the "before" for tickets 03, 06 and 08.
- Re-run the baseline once more and confirm two runs agree within criterion's
  noise, so a later "no change" means something. If the machine is noisy, say
  so here and fix the threshold the other tickets use.

## Done when

The baseline table is in this file, the new benches compile under `cargo bench
--workspace --no-run --locked`, and `./scripts/cargo-ci` passes.

## Comments

Partly done, not closed. The collide benches are written
(`crates/tesserae-core/benches/collide.rs`, registered in `Cargo.toml`) and
compile; scenes are a one-Chunk room of cubes or slabs, plus terrain.

**The baseline taken in this session is void and was deleted.** The machine's
load average was 54 (other sessions compiling and running `target/debug/
tesserae`), and the numbers it produced were roughly 8-10x the step-5 figures
(`shape_table_builtin` 42 ms against 4.8 ms, `greedy_mesh/terrain` 2.9 ms against
a few hundred microseconds), with confidence intervals of +-15% on the 500 ns
benches. A "no change" against that baseline would mean nothing.

Also learned: pass `--bench meshing --bench collide` to `cargo bench`, since a
bare `-- --save-baseline` is also handed to the lib's test harness, which
rejects it.

**Measured later, interleaved, and still not conclusive.** Other sessions kept
the machine at a load average of 30-100 for the whole branch, so a saved
criterion baseline could not be taken. Instead the `main` bench binary
(f232c4e, built in a detached worktree) and the branch's were run alternately,
five and then ten rounds each, so load hit both. Round to round the same bench
varied 2-3x. Medians of those runs, branch against main: `greedy_mesh/terrain`
+10.6% then -6.9%, `/solid` -8.5% then +3.8%, `/checkerboard` -8.7% then -1.5%,
`/terrain_stairs_at_cap` -2.4%, `/all_stairs` +2.5%, `mesh_chunk/straddling`
-2.5%, `light_chunk/straddling_settled` +7.1%, `/flooded_air` +2.2%,
`shape_table_builtin` -15.7%. The sign flips between runs of one bench, and
minimums swung from -38% to +40%, so none of this distinguishes a change from
noise. What can be said without a clean run: pass 1's code, `variant()` and
`seals` are untouched, a terrain Chunk never reaches the table or pass 2, and
the `Variant` struct grew by 10 bytes.

**Still to do, by whoever has a quiet machine:** `cargo bench -p tesserae-core
--locked --bench meshing --bench collide -- --save-baseline main` on f232c4e,
then `-- --baseline main` on this branch, with `uptime` load below ~2, and paste
the table here. The `collide` benches have no `main` side at all yet (the file
did not exist), which is why they need the first command run on a checkout of
this branch's `benches/collide.rs` against f232c4e's `collide.rs`.
