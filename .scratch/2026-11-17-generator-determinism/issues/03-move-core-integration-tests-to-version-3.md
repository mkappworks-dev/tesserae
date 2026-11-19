# 03: Move tesserae-core's integration tests onto version 3

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 01.

## What to build

Every test and bench in tesserae-core's integration tests that builds a World
does so under version 3. A test that depended on version 2's land (its gentle
heights, its single lake level, its Settlements on any ground) is rewritten
against version 3, or against a hand-built Chunk where the land itself is not
under test; none is deleted merely to pass. Tests of version 1 or 2 themselves
are left for 05.

## Acceptance criteria

- [x] No test or bench in this batch builds a World under version 1 or 2, other
      than tests of those versions themselves.
- [x] Every test that changed keeps asserting the behaviour it was written for;
      any whose subject only exists under version 2 is listed in a comment here
      for 05.
- [x] Generator output and the recorded digests are unchanged.

## Comments

- Every World these tests build starts from `testgen::world_params_v3()` with
  `water: None` wherever it used to inherit version 2's dry fixture. Version 3
  puts the main seed's origin on the sea floor at about -38 m, so a test whose
  subject stands on the ground rather than being its shape runs on
  `tests/common::lowland`: version 3 held between Sea level and `relief_m` above
  it, with no sea floor and no overhangs, which is version 2's band. A test that
  needs hills, a coast or a river scans a stretch a walk finds
  (`structure_worldgen::hills_m` and `coast_m`, `bridge_worldgen::centre_m`), or
  the Chunk the surface crosses (`meshing`).
- Rewritten for version 3's rules rather than its land: biome selection reads
  the Climate at the ground (altitude shifts it); a Structure fills caves within
  16 m of its footprint, so that change no longer counts as stray; a bridge's
  gap and wetness are asked per column, as the generator asks them, not against
  one lake level; overhangs are off where a test measures the ground under a
  Structure or a span; lakes are the sea over a let-down floor
  (`structure_worldgen::lake`, the kelp and near-water field tests).
- Recorded digests: version 2's are untouched and still pass, each beside a
  version 3 recording made in this change (`coarse_generation`,
  `surface_features`, `settlement_worldgen`'s fixed-size layouts). Version 3's
  coarse box hangs from the first ground east of the origin 10 m above Sea
  level, since no tunnel opens under the sea.
- For 05, tests of versions 1 and 2 themselves, left as they are:
  - `biome_ground::generator_version_1_ignores_biome_ground`
  - `biomes::generator_version_1_ignores_its_biomes`
  - `coarse_generation::coarse_generation_makes_the_recorded_version_2_chunks`,
    the version 2 half of `the_box_holds_caves_at_every_level_and_seed`, and the
    `2 =>` arms of `params` and `box_m`
  - `ore_veins::generator_version_1_has_no_ore`
  - `plant_worldgen::version_2_keeps_trees_declared_closer_than_their_crowns`
  - `settlement_worldgen::a_fixed_size_settlement_lays_out_exactly_as_before_ranges_in_version_2`
    and the `2 =>` arm of `fixed_size_digests`
  - `sky_fog::a_generator_from_before_biomes_has_none_to_send`
  - `strata::version_2_lays_no_strata`
  - `surface_features::a_version_2_world_whose_biomes_declare_no_features_makes_the_recorded_chunks`
  - `terrain_biomes::the_air_is_colder_higher_up_in_version_3_and_not_before`
    (its version 2 half)
  - `terrain_v3::version_2_keeps_its_old_bound`,
    `earlier_versions_spawn_where_they_always_did` and the version 2 half of
    `nothing_stands_above_a_footprints_own_top_and_versions_1_and_2_keep_the_global_bound`
  - `underground::version_2_has_no_lava_springs_or_cave_biomes`
  - `generator_version.rs`, whole.
- Found, not fixed (generation is out of scope): a surface feature on a
  column's top Voxel at a Chunk's top can be missing from the Chunk above. That
  Chunk's `still_solid` asks the raw tunnel field, which does not know version 3
  keeps tunnels shut beside the sea, so it reads the Voxel below as carved. Seen
  on 4 of about 6,000 columns of the lifted case of
  `a_feature_stands_on_every_full_surface_voxel_of_its_biome_and_nowhere_else`,
  whose lifted World is now dry to step round it.
- Also seen, not investigated: in the near-water fields test's window, 1 m
  Voxels stand fields within reach of water and 0.5 m Voxels stand none; the
  test asserts some near water across all sizes.
