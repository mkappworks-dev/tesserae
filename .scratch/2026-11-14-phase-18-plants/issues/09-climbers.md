# 09: Climbers

Status: done

Blocked by: 04, 03

Spec: [Phase 18](../spec.md).

## What to build

The `climber` kind: vines grow along the faces of solids as `plane` Shapes whose
Orientation names the face they hold, and hang below an overhang. A Sample that
passes adds a vine on an adjacent solid face, or below the last vine, within
`length-m`. A vine whose face-bearing solid goes breaks by the support rule.

## Acceptance criteria

- [x] A vine spreads along a wall and down an overhang to `length-m`, the same
      in metres at every Voxel size, and never through a solid.
- [x] It draws against its face in all four wall Orientations and hanging.
- [x] Removing the wall breaks the vine that held it, dropping its Seed or yield.
- [x] A body walks through a vine and a ray stops on it.

## Comments

- A vine takes the Shape of the vine that grew it; `ShapeTable::orientation_holding`
  picks the first of the 24 Orientations whose rotated first box lies against the
  needed face, and `ShapeTable::plane_face` reads that face back, so the mesher's
  own thin-axis rule decides it. Tested over all six faces.
- A Sample picks one of the six neighbours by hash. Below a vine on a wall face it
  copies that face (it hangs along the wall, or from the vine above where the
  wall ends); anywhere else it takes a face whose neighbour is `Needs.soil`, so
  an overhang gets a ceiling vine. A ceiling vine does not hang further down.
- `length-m` caps the vines face-connected to the Sampled one, in Voxels
  `ceil(length-m / voxel size)`, not a single run.
- Support: `PlantTable::lacks_support` reads soil on the face the Orientation
  names, or the same Material above; a Shape that is not `plane` falls back to
  `lacks_soil`, which now also counts the vine above. Removing the wall breaks the
  vine that held it and the ones hanging from it.
- "A ray stops on it" and "a body walks through it" are tested on the Shape
  table (empty collision, non-empty selection in all 24 Orientations), not on a
  ray cast.
