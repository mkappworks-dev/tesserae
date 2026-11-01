# 06: Quality probes for cheaper far levels

Status: ready-for-human
Blocked by: 04

Spec: [streaming performance](../spec.md).
PR boundary: probe branch `tech/streaming-far-probes`; nothing merges without
your approval of captures.

## What to probe

Two ways to make far levels cheaper that change what is drawn:

- **Direct generation**: a level-k Chunk sampled from the level-k generator at
  its own resolution instead of downsampling eight generated children. About
  8 times fewer samples; caves and overhangs survive, the majority vote does
  not.
- **Column-only**: from some level up, a height and surface Material per
  column, with no caves or overhangs.

For each: worker CPU per coarse Chunk against 04's result, and side-by-side
review captures at 256 m, 1 km and 2.5 km on the default seed (the demo's
LOD reach is 3000 m).

## Acceptance criteria

- [ ] Worker CPU per coarse Chunk and review captures for both probes recorded
      here.
- [ ] Your decision recorded under Comments. A probe never counts as an
      equal-quality improvement (frame-profiling.md).

## Comments

### 2026-11-01: both probes built, CPU and captures

Switch, read only in `dev` and `profiling` builds (`lod.rs`'s
`open_lod_generators`): `TESSERAE_FAR_PROBE=direct|columns`, with
`TESSERAE_FAR_PROBE_FROM=<level>` (default 2, so level 1, 160 to 320 m, is
untouched). Unset, `LodGenerators` holds no probe generator and every job takes
today's path: the test that every level is the downsample of its generated
children now runs through `FarProbe::Off`, and 04's digest tests are unchanged.

- **direct**: a level-k Chunk is `Worldgen::chunk` at a 2^k m Voxel, no
  smoothing, caves kept. The generator samples a Voxel's floor, which made a
  Voxel the ground barely enters ground and drained shallow lakes (first
  captures, `review/06v1-*`: dark under-water-lit pits at 1 km where the
  downsample draws ponds). The probe generator lowers the surface and the lake
  by half a Voxel, so it asks about the middle as the downsample's vote does;
  the lake by one ulp less, so it keeps its ties (the demo's 44 m lake sits on
  a level-3 Voxel's middle).
- **columns**: the same without caves: one height and the ground Materials per
  column, nothing carved.
- Either probe yields to the downsample for a job with a changed held child or
  saved changes, so an Edit under level 1 still shows (probe from 1).
- Tests (`server::lod`): probe content per level, finer levels untouched,
  columns solid-below-air in every column, no lake drained (at least 80% of the
  downsample's water at levels 2 and 3), Edit fallback, and the mesh shell
  delivered and fresh at every level with either probe, seam walls included.

Worker CPU per coarse Chunk, `coarse_breakdown -- 3` (its new second table:
the whole job as shipped, eight children plus the downsample, against the probe
generators over the same jobs), three runs, medians, us per job:

| Level | Jobs | Shipped     | direct   | columns | direct | columns |
| ----- | ---- | ----------- | -------- | ------- | ------ | ------- |
| 1     | 1639 | 145-148     | 20-21    | 5       | -86%   | -97%    |
| 2     | 1639 | 113         | 20       | 4       | -83%   | -97%    |
| 3     | 1639 | 107-108     | 15       | 3       | -86%   | -97%    |
| 4     | 1639 | 88-89       | 19       | 3       | -79%   | -97%    |
| 5     | 1243 | 106         | 8        | 3       | -92%   | -98%    |

Run ranges are under 2% wide and nowhere near overlapping. The engine agrees:
`capture_lod_cost gen_us` at levels 2 to 5 in the captures below is 124-181 us
off, 13-40 direct and 4-7 columns. Level fill time did not move (7.6 s off and
probed): level 1 and the generate budget bind it, not the far levels' cost.

Review captures, `stationary`, default seed, release `profiling` binary
`bin-06-mid` (97a7b56), probes from level 2. Grids are off | direct | columns:

- 256 m (`LIFT_M=45 PITCH_DEG=-10`): `review/06-grid-256m.png`
- 1 km (`LIFT_M=90 PITCH_DEG=-5`): `review/06-grid-1km.png`
- 2.5 km (`LIFT_M=120 PITCH_DEG=-2.7`): `review/06-grid-2500m.png`
- all three: `review/06-grid-all.png`; crops `review/06-crop-*.png`; runs
  `review/06-<distance>-<probe>/`. All under `/tmp/claude-streaming-impl/`.

My read:

- Near is untouched: level 0 and level 1 are bit-equal, and the 256 m
  foreground is identical across the three.
- From level 2 out, direct and columns are not the shipped picture, and the
  difference is visible: the terraces sit where the 2^k m surface puts them,
  sharper and taller-looking than the shipped double downsample; lakes fill
  their whole level-0 extent where the shipped chain loses ponds to its ties;
  the grey and brown speckle the vote leaves on slopes is gone. I read it as
  no worse and often cleaner, but it is a different picture, which is yours
  to judge.
- direct and columns look the same at every distance captured: past 320 m no
  cave is visible from above. columns costs a fifth of direct; a cave mouth
  past level 1 is what it gives up.
- Where level 1 meets level 2 the two disagree by up to half a coarse Voxel;
  the seam walls cover it and no gap shows, but a terrace step can.
- Neither counts as an equal-quality improvement (frame-profiling.md). The
  frame and streaming effect is pending the final review series.

Decision for you: adopt neither, direct, or columns, and from which level.
