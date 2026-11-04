# 03: The Server write batch

Status: done
Blocked by: 02

Spec: [bulk relight](../spec.md), decisions 2 and 3.

## What to build

`edit::Batch` in `server/edit.rs`: the one World change, split so the relight
can wait for the end of a batch.

- `Batch::set_occupants` and `Batch::set_voxel` do everything `set_occupants`
  does today except the relight: the `NotLoaded` check, air canonicalised,
  `vet_occupants`, the orientation dropped for cubes and connected Shapes, the
  shaped cap, the overlay and Voxel written, `note_changed`, `voxels_changed`
  and the edited and boundary-neighbour mesh marks. Where optics, wetness or
  burning changed, they record the position and the light it held before the
  batch.
- A write whose canonical Voxel and overlay equal what is stored returns
  `Ok(Written::Unchanged)` and touches nothing else.
- `Batch::finish` runs `relight_many` once over the recorded positions and
  marks every Chunk it wrote light into, and every held Chunk across a plane it
  wrote, once each. `Batch` is `#[must_use]`; nothing reads light between a
  batch's writes and its `finish`.
- `set_occupants` and `set_voxel` become a batch of one, so every existing
  caller (Edits, undo, Layouts, `dev` builders, fluids, Mods) keeps its
  behaviour; the doc comment's four kinds of stale mesh moves to `Batch`.
- If the shaped cap's whole-Chunk sum shows in 04's profile, a batch keeps a
  running count per Chunk instead.

## Acceptance criteria

- [x] Every existing Server test passes, one with a changed set-up (Comments).
- [x] A test applies the same mixed writes (solids, sources, fire, shaped,
      emitters, across a Chunk seam) one `set_occupants` at a time and as one
      batch, and the two stores hold equal Voxels, overlays and light, and
      `MeshState` holds equal stale and cover-changed sets.
- [x] A no-op write in a batch leaves the journal, the save revision and
      `MeshState` untouched.
- [x] A batch over the shaped cap refuses the write that would pass it, as one
      at a time does.

## Comments

### Result, 2026-11-04

`edit::Batch` holds `set_occupants`, `set_voxel` and `apply_intent`; the free
functions of those names are a batch of one each, so every caller kept its
code. One existing test changed:
`fluids::tests::full_detail_meshes_follow_the_water_as_it_flows_and_recedes`
took the Client's first mesh from a dry write of air over air, which now
changes nothing and delivers no mesh, so it takes that mesh from the `settle`
after the corridor is built.

The equivalence test fails if `finish` relights only the first write. The
shaped cap needs no running count: writes land in storage as they come, so a
batch's later write sees the earlier ones. Whether the whole-Chunk sum shows in
a profile is ticket 04's to see.
