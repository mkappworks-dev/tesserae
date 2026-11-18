# 02: Move tesserae-core's unit tests and benches onto version 3

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 01.

## What to build

Every test and bench in tesserae-core's unit tests and benches that builds a
World does so under version 3. A test that depended on version 2's land (its
gentle heights, its single lake level, its Settlements on any ground) is
rewritten against version 3, or against a hand-built Chunk where the land itself
is not under test; none is deleted merely to pass. Tests of version 1 or 2
themselves are left for 05.

## Acceptance criteria

- [x] No test or bench in this batch builds a World under version 1 or 2, other
      than tests of those versions themselves.
- [x] Every test that changed keeps asserting the behaviour it was written for;
      any whose subject only exists under version 2 is listed in a comment here
      for 05.
- [x] Generator output and the recorded digests are unchanged.
- [x] Bench figures before and after the switch are noted in a comment here;
      they are not comparable across it.

## Comments

Every World built by tesserae-core's unit tests and benches is now version 3, through `testgen::world_params_v3()`; no generation code or recorded digest changed.

Left on version 1 or 2 for 05, because their subject exists only there:

- `worldgen::tests::existing_seeds_keep_their_terrain`: version 2's terrain digest. `terrain_digest` now takes its parameters; the two seed-folding tests use version 3.
- `worldgen::tests::version_1_ignores_cave_tuning`: split out of `tuning_that_is_not_asked_for_changes_nothing`, whose version 3 half stays.
- `worldgen::tests::water_never_rises_above_the_highest_ground`: version 2's clamp of a lake level to the band; version 3's water stands at Sea level.
- `save::tests`: the "older worlds save none" halves of the three `version_3_*` tests, `a_version_1_world_and_an_empty_table_leave_the_snapshot_fields_absent`, `a_commit_written_before_biomes_restores_with_none` and `a_version_1_world_restores_with_none_whatever_its_snapshot_holds`. They build no World; they test the save's handling of versions 1 and 2. The other save tests that restored at version 2 incidentally now restore at 3.

Rewritten against version 3, each keeping its subject:

- The sweep round the origin is flat lowland under version 3 at `SEED`, so the tests that need sea, shore or one-Voxel steps there use a second seed, `COAST`.
- Water in version 3 holds back the overhangs and reshapes and re-materials the ground beside it, so `water_fills_the_open_air_between_the_ground_and_its_level_and_nothing_else` checks the wet World against its own surface and `water_surface_m` rather than diffing it with a dry one; `flooded` turns overhangs off.
- `the_default_parameters_have_no_lakes_and_generate_no_fluid` uses `WorldgenParams::default()` with the fixture ground, since the version 3 fixture declares water.
- Version 3's band reaches below the tunnels' floor and its caverns below that, so `nothing_below_the_cave_floor_is_carved`, `a_world_without_caves_is_what_it_was` and `the_range_test_keeps_deep_chunks_uniform_that_no_tunnel_crosses` look below the band.
- `light::tests` find their ramp, mouth and dry Chunks at each column's own surface height; `flow::tests::a_shaft_dug_under_a_generated_lake_fills_from_it` searches the Chunk rows under the water level.
- Benches: `meshing`'s `early_out_air`/`early_out_stone` Chunks come from `surface_bounds_m`, as they no longer were early-outs; `fluids/rescan` finds its shore at the water level's row and its dry World has no water; `structures` anchors its scan on `settlement_spawn_m`, since version 3 founds Settlements about a kilometre apart.

Observed, not changed (generation is out of scope here):

- At seed 3907 with overhangs on, smoothing places a stair that climbs two steps and a slab easing fewer than two (`assert_ramps_are_whole`, `smoothing_stairs_face_the_step_they_climb`); the smoothing tests keep `SEED`, where both hold. Worth its own ticket.
- `lod/coarse_chunk/air_level_1` is no longer an early-out under version 3, and a cold generator costs several times a warm one in `structures`.
- `coarse_breakdown` and `structures` still set `relief_m`, `feature_size_m` and lake levels, which version 3 ignores; 05 can drop them with the version 2 parameters.

Bench figures, indicative only: the host was loaded, and the load fell between the runs, so even unchanged early-outs read about 4x faster after. Criterion `--quick` medians; before on version 2 at ae958f4a (load ~45), after on version 3 (load ~20).

| Bench | Before (v2) | After (v3) |
| --- | --- | --- |
| `worldgen/straddling` | 799.53 µs | 240.52 µs |
| `worldgen/shore` | 397.63 µs | 131.03 µs |
| `worldgen/early_out_air` | 41.577 ns | 8.6780 ns |
| `worldgen/early_out_stone` | 47.430 ns | 10.832 ns |
| `worldgen/cave_crossed` | 794.25 µs | 575.60 µs |
| `mesh_chunk/straddling` | 2.2739 ms | 369.39 µs |
| `light_chunk/straddling` | 855.87 µs | 65.510 µs |
| `lod/coarse_chunk/surface_level_1` | 7.5137 ms | 1.5217 ms |
| `lod/coarse_chunk/air_level_1` | 416.01 ns | 129.28 µs |
| `lod/coarse_chunk/rock_level_1` | 672.13 ns | 115.25 ns |
| `fluids/rescan/lake_shore` | 40.118 ms | 7.8844 ms |
| `fluids/rescan/dry_terrain` | 54.216 ms | 12.224 ms |
| `structures` bare, 1 m, per Chunk (load ~46 / ~16) | 454.09 µs | 142.53 µs |
| `structures` bare, cold, 1 m, per Chunk | 450.80 µs | 3490.52 µs |
| `plants` 16 features off, 1 m, 9 Chunks | 2382.4 µs | 1164.6 µs |
