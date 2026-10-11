# Docs, ADR, measurements, screenshot

Status: ready-for-agent
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
