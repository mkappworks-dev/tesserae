# 03: The Server write batch

Status: ready-for-agent
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

- [ ] Every existing Server test passes unchanged.
- [ ] A test applies the same mixed writes (solids, sources, fire, shaped,
      emitters, across a Chunk seam) one `set_occupants` at a time and as one
      batch, and the two stores hold equal Voxels, overlays and light, and
      `MeshState` holds equal stale and cover-changed sets.
- [ ] A no-op write in a batch leaves the journal, the save revision and
      `MeshState` untouched.
- [ ] A batch over the shaped cap refuses the write that would pass it, as one
      at a time does.

## Comments
