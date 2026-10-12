# Place stairs and slabs where the ground steps one Voxel

Status: done

Spec: the rule, decisions 1 to 4 and 6.

## What to build

- `SmoothingParams { min_rise_m }` (default 0.75) and
  `WorldgenParams::smoothing: Option<SmoothingParams>`, default `Some`.
  Re-exported from the crate root with `CaveParams`.
- `Worldgen::chunk`: sample a one-column border of surface heights, and after
  the cube pass run the smoothing pass per the rule. Topmost index per column
  comes from the same comparison the cube pass uses. A stair when exactly one
  cardinal neighbour rises one Voxel, a slab when two or more do, Material
  Grass. The pass runs only if `(voxel_size)` clears `min_rise_m`, compared in
  metres.
- A solid-cube check on the supporting cube and on each rising neighbour's top
  cube, using `CaveField::carves` for cells that may be carved (a cell outside
  the Chunk is asked by its metre position, not read from a neighbour).
- One function from a direction to a stair `Orientation`.
- `surface_bounds_m` doc: a stair is solid and sits inside the band.
- Test helpers `terrain()`/`caved()` set `smoothing: None`; add `smoothed()` and
  `smoothed_caved()`. Pin `smoothing: None` in light.rs's `cave_worlds()` and in
  the bench cave fixtures.
- Remove no existing test silently.

## Acceptance criteria

- For all four directions the placed stair seals the face towards the rising
  neighbour and not the opposite one (checked against the built `ShapeTable`).
- Every shaped Voxel sits on a solid cube and abuts a solid cube at its own
  level, with caves on, over a sweep that includes cave mouths and Chunk edges
  (cells looked up across Chunks).
- Two Chunks agree on a shared edge; a stair whose cell is in the Chunk above
  appears there and not in the Chunk below.
- Generation is deterministic.
- `min_rise_m` is metres: at 1.0, 0.5 and 0.25 m Voxels the pass fires exactly
  when `voxel_size >= min_rise_m`, with two thresholds.
- The sky and deep early-outs still return uniform Chunks over a sweep, and no
  shaped Voxel is above the band.
- With `smoothing: None` every existing worldgen test holds unchanged.
- `terrain_output_is_pinned` passes; `testgen` is not edited.

## Comments

Built as specified, with two corrections from review: `axis_m` computes a
Chunk-edge position from the Chunk that owns it, exactly as that Chunk does
(`c * size * voxel_size + l * voxel_size`), because the other spelling differs
in the last bit at Voxel sizes that are not a power of two and a stair on an
edge is decided about a column in the next Chunk
(`two_chunks_agree_on_a_ramp_at_a_voxel_size_that_is_not_a_power_of_two`, at
0.3 m); and the surface border is sampled only when smoothing runs, so
`smoothing: None` costs nothing.

- The turn mapping is checked against the built `ShapeTable`. Swapping two
  turns in `CARDINALS` makes `smoothing_stairs_face_the_step_they_climb` fail.
- A cave suppresses ramps (`a_ramp_is_never_cut_by_a_chunk_edge_or_floating_over_a_cave`
  counts stairs the caved World lacks and the cave-free one has).
- Every stair climbs exactly one one-Voxel step: a cube beside it at its level
  with no cube above, counted over the sweep.
