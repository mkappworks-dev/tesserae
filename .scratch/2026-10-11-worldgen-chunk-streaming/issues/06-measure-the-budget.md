# Measure the per-tick budget, and update the docs

Status: done
Blocked by: 05

DESIGN.md calls the per-ring Chunk budget "a number to measure, not to guess",
and ticket 04 lands with a guess and a `TODO` pointing here. This ticket
replaces it with a measurement and closes the step out.

## Task

### Measure

Run `cargo bench -p tesserae-core` on the release profile and record, in the
PR:

- the step 2 baseline — terrain ~360 us, solid ~339 us, checkerboard ~2.46 ms,
  worldgen ~114 us — against the same benches now, to show the noise move and
  `mesh_chunk` cost the mesher nothing
- the new `worldgen` group, straddling and both early-outs
- the two `mesh_chunk` fast paths
- that the renamed `testgen` bench is a rename, not a regression

Then size the budget from those numbers against a tick at 64 Hz, and write the
chosen values into `StreamingBudget`'s `Default` with a comment giving the
measurement they came from and the date. Drop the `TODO`.

Sanity-check it in the running app rather than only on paper: fly at the
camera's top speed and confirm no hitch. `cargo dev` is a debug build with
`opt-level = 1`, so read it as a lower bound, and say which profile the number
came from.

### Docs

- `DESIGN.md`: the per-ring budget under "Open items" has a measured number
  now. Record it, and leave the item open if the measurement is only for one
  Voxel size and one view distance — which it will be. Say which.
- `docs/architecture.md`: the layout block gains `worldgen.rs` and `noise.rs`
  under `tesserae-core/src/` and the `server/` files, each with its build-order
  marker. `server/` is already marked step 3.
- Both files are Prettier-formatted: run `npm run fmt` and do not hand-wrap.

### Close the tickets

Set `Status: done` on 01 to 06 in the commit that finishes each, per
`docs/agents/issue-tracker.md`.

## Done when

The full gate passes: `cargo test --workspace`, `cargo lint`,
`cargo fmt --all --check`, `npm run fmt:check`,
`cargo deny check advisories licenses bans sources`, and
`cargo bench -p tesserae-core` with no regression. A screenshot of streamed
terrain is in the PR.

## Comments
