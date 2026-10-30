# 01: Layout geometry in tesserae-core

**What to build:** a pure, Bevy-free function that expands a layout into an
ordered list of Voxel coordinates: anchor, signed axis per layout axis, counts
per axis (1 to 3 axes, so line, rectangle and box) and a gap in Voxels. A
layout over 256 Voxels is refused. Server and Client will both call it.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

Spec: [layout placement](../spec.md).

- [ ] Line, rectangle and box produce the expected ordered coordinates, for
      every signed axis combination.
- [ ] Gap 0 is adjacent; gap n leaves n Voxels between neighbours.
- [ ] A layout above 256 Voxels is refused, at the boundary exactly 256 is
      accepted.
- [ ] The result depends only on Voxel counts, never on Voxel size.
- [ ] `tesserae-core` gains no Bevy or wgpu dependency.
