# 20: Two Server tests fail with the dev feature

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

Ticket 05 ran the whole workspace with CI's `tesserae/dev` feature and found two
failures that also fail on the commit before it (ticket 05's report); earlier
tickets ran `-p tesserae` without the feature and did not see them:

- `server::gallery::tests::parts_follow_properties_and_missing_kinds_are_omitted`
- `server::streaming::tests::every_mesh_carries_the_sight_lines_of_its_chunk_as_it_stands`

Find whether each fails on main or was introduced on this branch (and by which
ticket), and fix it so CI's Test job passes.

## Acceptance criteria

- [ ] Each failure's cause, and the first commit it fails on, are noted in a
      comment here.
- [ ] Both pass under `cargo test --workspace --features tesserae/dev`,
      unweakened.
- [ ] No recorded digest moves.
