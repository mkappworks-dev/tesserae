# Leaf holes: one definition, less regular

Status: done
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

Built as specified. `leaf_hole` lives once, in `render/leaf_hole.wesl`, which
both stages import (`import super::leaf_hole::leaf_hole`). The module has to be
loaded to be found: registered with `embedded_asset!` alone, nothing held a
handle to it and every chunk pipeline failed to compile ("no shader registered
for this module path"), so it is loaded with `load_shader_library!`, as
`bevy_core_pipeline` loads its OIT module. The hash is PCG's output permutation
over the quarter-Voxel cell, in place of a sum of multiplied coordinates: the
gallery's canopy shows no rows or stripes, and its shadow is dappled.
