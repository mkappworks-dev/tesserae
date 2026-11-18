# 20: Two Server tests fail with the dev feature

Status: done

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

- [x] Each failure's cause, and the first commit it fails on, are noted in a
      comment here.
- [x] Both pass under `cargo test --workspace --features tesserae/dev`,
      unweakened.
- [x] No recorded digest moves.

## Comments

Both pass on main and at ae958f4a and first fail at 2d143ee6 (ticket 04), which
moved them onto `world_params_v3` but ran `-p tesserae` without `dev`, under
which both are compiled out.

- `parts_follow_properties_and_missing_kinds_are_omitted`: the sparse World
  spreads the version 3 fixture parameters (`world_params_v3` at ticket 04,
  `world_params` since ticket 05), which carry a sea, so `Parts::select` finds
  a lake fluid the test means to be missing. The sparse World now sets
  `water: None`; the assertions are unchanged.
- `every_mesh_carries_the_sight_lines_of_its_chunk_as_it_stands`: version 3
  puts the default seed's origin far out at sea, so Chunk (0, -1, 0) is water
  that already connects every face, and the tunnel cannot change its sight
  lines. The test now uses `Harness::on_land`, like ticket 04's other streaming
  tests, where that Chunk is solid ground.

Test-only changes; no digest is involved.
