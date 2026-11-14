# 03: `cross` and `plane` Shape forms

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 18](../spec.md).

## What to build

A Shape can be a `cross`, two diagonal quads through the Voxel's centre, or a
`plane`, one quad against a face, in addition to nodeboxes. This is the
amendment to [ADR-0006](../../../docs/adr/0006-shapes-are-table-data.md). Both
forms are non-solid for bodies (collision passes, as through an open Door),
may seal no face, cannot connect, and cost nothing against the Chunk's
shaped-Voxel cap, and stay selectable so rays stop on them. Their boxes still
decide selection and fluid, so kelp is waterlogged where it grows; light passes
by the Material's own dimming. Meshing draws them in the cutout pass from
the Material's texture at level 0; coarse levels leave them out. A `plane`
attaches to the face its Orientation names.

## Acceptance criteria

- [ ] A Mod declares a `cross` and a `plane` Shape and places a Voxel of each;
      a body walks through it and a ray stops on it.
- [ ] It dims no light and hides no neighbour face; a stone cube beside one
      still draws its face.
- [ ] The mesher draws it without merging quads across it, in all 24
      Orientations for `plane`.
- [ ] A flat Shape that seals a face, or connects, is a load error; its boxes
      still give waterlogging, and it costs nothing against the shaped cap.
- [ ] Voxel size does not change its drawn extent in metres.
- [ ] A World with only box Shapes meshes and lights identically to `main`.
