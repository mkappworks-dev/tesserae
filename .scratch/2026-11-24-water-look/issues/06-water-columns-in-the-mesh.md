# 06: Water columns in the mesh

Status: ready-for-agent
Blocked by: 05

Spec: [Water that looks natural](../spec.md), Water columns.

## What to build

Every wet Chunk's mesh carries its water columns: per column of the Chunk and a
one-column apron, Flow, depth in metres and foam and Fall flags, about 4 bytes
each. A `tesserae-core` function builds them from the lit Chunk, its neighbours
and a trait of water answers that `Worldgen` implements; the Server's mesh job
calls it, and so gains the Generator as an input.

Each column is classified on its own. **Generated:** its top water Voxel is a
Source of the World's water at the Voxel the Generator's Water surface tops out
in; it takes the Generator's Flow and Fall, and depth from that drawn top to the
generated ground. **Every other column** (poured, simulated, dammed, a
Settlement pond): Flow down the surface gradient, falling Fluid flagged, depth
scanned as far as the job sees and saturating. Diagonal apron columns take the
Generator's answers where generated and otherwise repeat the nearest held
column. Dry Chunks carry none.

This ticket ends at `ChunkMesh` arriving on the Client. Uploading and drawing it
is ticket 10, after the rebase, because the Client's chunk mesh code changes on
the colour branch.

## Acceptance criteria

- [ ] Core tests through the builder, with a test double for the water answers:
      a dry Chunk has no water columns; a generated lake's depth runs from its
      drawn top to its ground; simulated flow points down the surface gradient;
      falling Fluid is flagged; a dammed river column and a Settlement pond take
      the scan rule.
- [ ] Every existing fluid meshing test passes unchanged.
- [ ] Mesh job time on the river and coast fixtures against the commit before,
      in Comments.
