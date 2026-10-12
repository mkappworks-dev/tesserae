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
