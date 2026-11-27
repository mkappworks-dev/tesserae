# 06: Water columns in the mesh

Status: done
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

- [x] Core tests through the builder, with a test double for the water answers:
      a dry Chunk has no water columns; a generated lake's depth runs from its
      drawn top to its ground; simulated flow points down the surface gradient;
      falling Fluid is flagged; a dammed river column and a Settlement pond take
      the scan rule.
- [x] Every existing fluid meshing test passes unchanged.
- [x] Mesh job time on the river and coast fixtures against the commit before,
      in Comments.

## Comments

- 2026-11-26, on e0d169eb plus this ticket:
  - `tesserae_core::water_columns::water_columns` builds them; `ChunkMesh`
    carries them as `water`, 34 x 34 entries of 4 bytes (Flow in 1/31.75 m/s
    steps, depth in 1/16 m steps saturating at 16 m, `FALLING` and `FOAM`
    flags), or none for a Chunk with no fluid. The Server's mesh job passes it
    the Generator and the fluid table.
  - Generation packs its water as cubes of the Material, not in the overlay,
    so the scan reads each Voxel as the fluid simulation does
    (`Occupants::of`): a packed cube is a Source.
  - A generated column's foam is a Fall over a neighbour landing toward it.
    A simulated column is falling when it descends, or when its water runs up
    past what the job sees and is not a Source.
  - Apron corners repeat the held column beside them along z where the
    Generator has no water there.
  - Cost, in release on terrain_v3's land, 46 wet Chunks over brooks and Falls
    (load 3): 0.27 ms to mesh one and 0.32 ms for its water columns. The first
    version asked the drainage some 19 times a column, 1.4 ms; the answers now
    read the Cell's column cache, and only the Flow walks the drainage.
  - The mesh job time on the river and coast fixtures waits, with ticket 01's
    series, for a quiet host.
- 2026-11-27, mesh job time on the river and coast fixtures, release
  `profiling` builds of 981fd487 (before) and 51bca61d (this ticket), three
  rounds each through `scripts/profile-frames`, load 3-7 (the runs' own):
  - Level 0 mesh job, whole run: river 0.60 to 0.81 ms a mesh (+0.21 ms), coast
    0.40 to 0.48 ms (+0.08 ms), over about 950 and 1 100 meshes a run. River
    Chunks are wetter, as the in-test 0.32 ms a wet Chunk predicted.
  - `mesh_worker_ms_per_s` over the fixture window: river 4.0 to 5.7, coast
    10.3 to 12.1, on worker threads.
  - Median frame: coast 4.21 to 4.23 ms; river 13.2 to 14.0 ms (rounds 12.9,
    13.3, 13.2 against 13.2, 14.0, 14.0), the main thread's wall time up with
    it. The Client does not read the columns until ticket 10, so the river
    step is not explained by this ticket's work; ticket 16's interleaved budget
    run measures river again.
