# Cheaper fence geometry, so more of them fit in a Chunk

Status: needs-triage
Blocked by: 07

Spec decision 10's outcome. A Chunk of 1024 reference fences meshed in 5.2 ms
against 1.5 ms for 1024 stairs, so a connected Shape is charged `cap_units`
(7 stairs for the fence) and 146 fit in a Chunk. That protects the mesh budget
and caps what a player can build: a fenced field hits it.

## What to decide

The cost is quads, and a fully armed fence emits several times a stair's: each
arm is two rails, and each rail is its own box with side, top, bottom and end
quads, and the post's side faces are split around where the rails join them.
Two levers, which are not exclusive:

- **The reference fence.** One flat slat per arm instead of two rails, or
  rails that stop short of the post's faces, emits fewer quads. This changes
  how the fence looks and the numbers in `shape::examples`, `meshing.rs` and
  the gallery, and nothing in the engine. Mods own their own Shapes, so this
  helps the reference only.
- **The engine.** `extract_quads` merges only within one plane of one Shape.
  Rails of neighbouring fences are collinear and coplanar and could merge into
  one long quad per rail across a fence line, which is also what pass 2's
  per-Voxel emission forbids ("nothing merges across Voxels"). That is a real
  change to pass 2 and to the hole-free property, so it wants its own spec.

## Acceptance

- `cap_units` for the reference fence drops (target: 4 or fewer) without the
  arms looking different in the gallery capture, or the engine change is
  specified and the property test extended to it.
- `terrain_fences_at_cap` at the new cap meshes within +50% of
  `mesh_chunk/straddling`, as the cap's rule says.
- The weight is derived, not hand-set: change the geometry and `cap_units`
  follows.
