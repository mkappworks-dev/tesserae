# Docs, ADR, measurements, screenshot

Status: done
Blocked by: 01, 02, 03, 04, 05, 06, 07, 08

## What to build

- `CONTEXT.md`: Nodebox, Sealed face, Edit; sharpen Orientation.
- `docs/adr/0006-shapes-are-table-data.md`: a Shape is a row in a table; the
  ShapeId-to-definition mapping is save data, with permanent built-in ids;
  Orientation is one of 24 rotations in 5 bits. The two things that are
  expensive to change once a save exists.
- `DESIGN.md`: step 5 marked done with what was built; the "Shaped voxels"
  section records the four consequences as resolved, the LOD rule and the cap;
  the open items gain the loose ends and the per-ring budget note is updated if
  the worst tick moved. `docs/architecture.md`: layout block for `shape.rs`,
  `collide.rs`, `server/edit.rs`, `client/edit.rs`, the diagram's `COLL` node,
  and the ADR index line. `CONTRIBUTING.md`: the controls.
- `StreamingBudget::default`'s comment re-derived from the new bench numbers.
- Run `cargo bench -p tesserae-core` against the step-4 table and write before
  and after into the PR.
- The `dev` capture screenshot of the gallery, taken and shown.
- The PR description, filled in from the template with the unrelated items
  struck through, given in full in the session.

## Done

`./scripts/cargo-ci` passes in full, and the spec's "Done when" holds.

## Comments

Final numbers against the step-4 baseline saved on this machine the same
morning (`--save-baseline step4`): `greedy_mesh/terrain` -12.0%, `/solid` -9.3%,
`/checkerboard` -0.2%, `/terrain_light_gradient` -11.5%, `mesh_chunk/straddling`
-11.1%, `light_chunk/flooded_air` -1.5%, `/straddling_settled` +1.5%,
`/straddling` +12.0% (about 10 us), `/open_sky` +1.3 ns and `/opaque` +0.8 ns.
Everything else is within noise. The worst streaming tick re-derived from them
is 8.1 ms against 15.6 ms, down from 8.6.

In the app, four runs after the fresh build: worst frame 24.5 to 43.9 ms,
2167 Chunks loaded and 791 meshed, as in step 4. Two earlier runs, the first
after a fresh build, showed 250 ms, which is Bevy's frame-delta clamp; they did
not reproduce in the four that followed.

`cargo-deny` prints two `duplicate` warnings for `thiserror` 1 and 2. They are
in the lockfile on `main` and this branch adds no dependency.
