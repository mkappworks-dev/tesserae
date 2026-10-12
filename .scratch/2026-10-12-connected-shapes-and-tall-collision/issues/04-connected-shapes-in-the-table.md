# Connected shapes in the ShapeTable

Status: ready-for-agent
Blocked by: 02, 03

Spec decisions 2-5. The table only: nothing meshes, sweeps or picks a connected
Shape yet, and no caller changes behaviour.

## What to build

- `ConnectDef`, `Arm`, `ShapeDef::connect`, `Connections` (a `u8` newtype of
  face-constant bits).
- `register` validation, each its own `ShapeError`: arm face out of range or
  duplicated; arm boxes out of range; `MAX_BOXES` over base plus all arms (and
  likewise for collision); **`ConnectedSeals`** if any submask seals a face;
  **`ArmTouchesOtherFace`** if an arm's boxes reach a boundary plane other than
  its own face's.
- Registration derives a variant per submask of the arms from the one
  rasterised grid (base plus selected arms, unrotated): quads, coverage,
  volume, collision, selection. A connected Shape's 24 orientation slots are
  copies of its mask-0 variant, so `variant()` and `seals` are unchanged for
  every Shape.
- `ShapeTable::connections(voxel, around: [Voxel; 6]) -> Connections` per spec
  decision 3, and `is_connected(voxel)`.
- `quads`, `collision` and `selection` take a `Connections`; callers outside
  this ticket pass `Connections::NONE` until 05 and 06 change them.
- `coverage_against(voxel, face, other: Voxel)` for decision 6.
- `volume` is the mask-0 variant's.

## Tests

- A fence (post + four arms, two rails each) registers; its 16 variants have
  arms exactly where the mask says, in `quads`, `collision` and `selection`.
- `connections` truth table: fence beside fence (same group), a different
  group, a cube with `to_sealed`, a slab (sealed one side only: connects from
  that side), air, an unknown Shape, a Shape that connects asymmetrically.
- Registration rejects each invalid case above, including a "fence" with a
  full-height solid arm that would seal.
- A connected Voxel's orientation byte is ignored for any value, including 24+.
- `coverage_against` equals the coverage of the variant with only that bit set,
  for every face and every connecting/non-connecting neighbour. (This pins the
  decision-4 invariant, not just one fence.)
- Registration time of a fence and of a six-way pipe, as a bench.
- Every existing `shape.rs` test passes unchanged.

## Done when

`./scripts/cargo-ci` passes; `shape_table_builtin` and the pass-1 benches from
01 are unmoved.
