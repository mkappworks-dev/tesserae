# Light and occlusion are right on smoothed terrain

Status: done
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

- `a_ramp_on_open_ground_agrees_between_solvers_and_casts_no_shadow`: the ramp
  placed Voxel by Voxel agrees with the bulk solve and with the cube terrain's
  light exactly, and so does removing it again.
- `a_ramp_beside_a_tunnel_mouth_agrees_between_solvers`, with sky dimming into
  the tunnel.
- `no_exposed_face_of_generated_ramps_is_missing_or_drawn_twice`
  (tests/meshing.rs): sub-cell ground truth round shaped Voxels of real Chunks,
  including ones on a Chunk seam, against the neighbours' real Voxels.
- `declining_never_changes_the_mesh_of_a_ramp_chunk`.
- Not mutation-tested: the hole check and the two light tests were not shown
  failing against a deliberately broken mesher or solver. The existing random
  scene test and the light agreement tests they extend were.
