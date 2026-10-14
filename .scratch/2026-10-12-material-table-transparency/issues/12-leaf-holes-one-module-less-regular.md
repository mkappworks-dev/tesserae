# Leaf holes: one definition, less regular

Status: ready-for-agent
Blocked by: 10

Ticket 10's hole pattern reads as rows and stripes, and `leaf_hole` is written
twice, in `chunk_material.wesl` and `chunk_prepass.wesl`, kept in step by a
comment.

## What to build

- One `leaf_hole`, in a shader module both stages import, if WESL imports
  between the engine's embedded shaders resolve; otherwise the two copies stay
  and the ticket says why.
- A hash that mixes the cell and the Voxel well enough that a canopy does not
  show rows.

## Acceptance criteria

- A capture of the gallery's canopy without visible rows or stripes; its
  shadow still has gaps.
- `./scripts/cargo-ci` passes.

## Comments
