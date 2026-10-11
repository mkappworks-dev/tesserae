# Docs, vocabulary and the measurements

Status: ready-for-agent
Blocked by: 05, 06, 07

## What to change

- `CONTEXT.md`: add **Light level**, **Sky light** and **Block light** under a
  heading the glossary's structure wants - they are properties of a position in
  space, so they sit with World rather than with Content. A light level is
  dimensionless on purpose; say so, because every other number in that file is
  in metres and the next reader will ask.
- `DESIGN.md`: tick step 4 in the build order. Update "Open items" - the
  per-ring Chunk budget item carries the measured worst tick, and this step
  adds a third term to it.
- `docs/architecture.md`: the layout block already reserves
  `tesserae-core/src/light.rs` for step 4; no change needed there, but check
  the `LIGHT` node's description in the diagram still reads true.
- `crates/tesserae/src/server/mod.rs`: register the `dev` lightbox module.

## The numbers the PR has to carry

Step-3 baselines on this machine, Apple M5, release,
`cargo bench -p tesserae-core`: `greedy_mesh/terrain` 358.4 us, `solid`
338.1 us, `checkerboard` 2.32 ms, `worldgen/straddling` 102.2 us, `worldgen`
early-outs 4 ns, `mesh_chunk/straddling` 371.1 us, `mesh_chunk` declines
~5 ns, `neighborhood_isolated` 22.5 us.

Re-measure all of them, plus the new `light_chunk` cases, and put before/after
in the PR with an explanation for anything that moved.

**Then re-derive the worst streaming tick.** It was
`24 x 102 us + 12 x 371 us = 6.9 ms` against the 15.6 ms a tick has at 64 Hz.
Relight adds a third term on the same tick. If the sum no longer fits, change
the budget and say so in the PR rather than leaving the comment on
`StreamingBudget::default` describing a tick that no longer exists.

Also confirm it in the app, as step 3 did: `TESSERAE_SCREENSHOT` reports the
worst frame since start-up, and that number belongs in the PR beside the
screenshot.
