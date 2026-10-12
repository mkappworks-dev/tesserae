# Light and occlusion are right on smoothed terrain

Status: ready-for-agent
Blocked by: 01

Spec invariant 5. This is the first generated input with shaped Voxels and
caves together, so the ride-along tests are not enough to trust.

## What to build

- Bulk and incremental light agree on smoothed and smoothed-and-caved terrain:
  extend the existing `light.rs` agreement tests (the generated-terrain section)
  to use `smoothed`/`smoothed_caved` worlds, across a Chunk boundary where the
  ramp crosses it.
- The brute-force hole check from `tests/meshing.rs`
  (`no_exposed_face_of_a_random_scene_is_missing_or_drawn_twice`) run on a real
  neighbourhood of generated terrain: build the 1/16-resolution occupancy of a
  Chunk plus its neighbours, mesh the middle Chunk, and assert every exposed
  sub-cell face is drawn once.
- `declining_never_changes_the_mesh_it_would_have_produced` on smoothed terrain
  by name, not by riding on the default.
- Check that `seed_from_border` and the open-sky shortcut are exercised: the
  test must contain a Chunk whose +Y neighbour may contain shaped Voxels.

## Acceptance criteria

- Each test fails if the property it names is broken (state in Comments how that
  was checked, e.g. by temporarily reversing a turn).
- No existing light or meshing test is weakened.

## Comments
