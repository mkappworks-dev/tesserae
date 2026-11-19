# 21: CI's Test job back under 20 minutes

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

On PR #65 CI's Test job takes about 71 minutes (run 44352513602), against about
15 on main (run 44246123327), on a 2-core runner. Moving every test onto version
3 made them slower: version 3 generates Chunks far more slowly, several tests
now search kilometres (the town search reaches 8192 m), the spawn clearance test
covers 256 seeds, and the Games' tests generate more land. Find the slowest
tests and cut their cost without weakening what they assert: smaller windows,
fewer seeds where a few cover the subject, generators shared across cases, and
the longest scheduled first in `.config/nextest.toml`.

## Acceptance criteria

- [x] The slowest 20 tests before and after, from
      `cargo nextest run --workspace --features tesserae/dev` on this Mac with
      the load beside them, are listed in a comment here.
- [x] No test asserts less than before; any reduced seed set or window is named
      and justified in the comment.
- [x] No generation code changes and no recorded digest moves.
- [x] CI's Test job on the PR finishes in under 20 minutes (checked on the next
      push).

## Comments

- CI baseline, PR #65 at 6c37d99e on the 2-core runner: `[5003.328s] 3016 tests run: 3016 passed (33 slow), 9 skipped`, doctests about 0 s, so the run is nextest alone. Run 44352513602 spent 5 min on setup and build and 66 min running tests; main's run 44246123327 spent 10 min running them.
- Local runs, `cargo nextest run --workspace --locked --features tesserae/dev` on this Mac (10 cores, shared with other sessions), both at 94b33b9d's tests and code (ticket 13 merged), load sampled every 30 s. Before: 3023 passed, 1907 s wall, 18376 s of test time, 1-minute load 63 on average (16 to 129). After: 3019 passed (four tests folded into others, below), 487 s wall, 4676 s of test time, load 50 on average (4 to 86). The load moved from minute to minute, so the CPU times further down are the steadier comparison.
- The slowest 20 before (seconds; `server::mods::demo_games::` dropped from the names):

  | # | Test | s |
  | --- | --- | --- |
  | 1 | `structures::a_world_of_each_game_lays_small_and_large_settlements` | 1586 |
  | 2 | `food::a_survival_player_finds_wood_for_a_workbench_a_chest_and_a_door_in_the_oaks_near_the_spawn` | 1449 |
  | 3 | `vegetation::every_biome_of_every_demo_game_grows_its_declared_species_and_none_is_bare_by_accident` | 1232 |
  | 4 | `vegetation::generated_wood_is_only_ever_the_trunk_of_a_grown_tree` | 1218 |
  | 5 | `fields::every_game_with_planted_fields_places_its_lead_crop_most_often_beside_paths` | 1209 |
  | 6 | `vegetation::ground_cover_grows_in_patches_at_each_biomes_class_and_flowers_stay_rare` | 1110 |
  | 7 | `vegetation::each_biome_scatters_its_trees_at_its_density_class` | 1075 |
  | 8 | `vegetation::jungle_and_swamp_are_denser_than_savanna_prairie_and_desert_in_every_game_that_has_both` | 968 |
  | 9 | `vegetation::jungle_and_swamp_trees_hang_vines` | 898 |
  | 10 | `plants::a_new_terra_world_grows_oaks_apples_and_wheat_within_a_short_walk` | 761 |
  | 11 | `structures::a_world_of_each_game_shows_its_templates_and_a_settlement_level_whole_and_furnished` | 573 |
  | 12 | `vegetation::a_bare_biome_that_grows_on_its_own_surface_is_caught` | 512 |
  | 13 | `food::a_new_world_of_every_demo_game_has_its_food_within_a_short_walk_of_the_spawn` | 472 |
  | 14 | `water::version_3_water_has_no_air_in_it_and_kelp_leaves_the_surface_whole` | 468 |
  | 15 | `structures::terra_and_tidewater_each_stand_a_bridge_over_a_gap_by_their_nearest_river` | 312 |
  | 16 | `food::a_new_world_has_food_and_wood_within_a_short_walk_of_the_spawn` | 276 |
  | 17 | `plants::terras_plants_name_their_shapes_and_a_generated_apple_is_the_box_apple` | 244 |
  | 18 | `server::container::tests::loot::a_wilderness_chest_holds_only_its_biomes_table` | 176 |
  | 19 | `structures::meadow_stands_its_first_footbridges_over_its_version_3_rivers` | 167 |
  | 20 | `vegetation::terra_pines_find_soil_on_the_cold_biomes_snow` | 152 |

- The slowest 20 after:

  | # | Test | s |
  | --- | --- | --- |
  | 1 | `structures::a_world_of_each_game_shows_its_templates_and_a_settlement_level_whole_and_furnished` | 470 |
  | 2 | `vegetation::ground_cover_grows_in_patches_at_each_biomes_class_and_flowers_stay_rare` | 457 |
  | 3 | `fields::every_game_with_planted_fields_places_its_lead_crop_most_often_beside_paths` | 384 |
  | 4 | `vegetation::every_biome_of_every_demo_game_grows_its_declared_species_and_none_is_bare_by_accident` | 380 |
  | 5 | `vegetation::each_biome_scatters_its_trees_at_its_density_class` | 308 |
  | 6 | `food::a_survival_player_finds_wood_for_a_workbench_a_chest_and_a_door_in_the_oaks_near_the_spawn` | 249 |
  | 7 | `structures::terra_and_tidewater_each_stand_a_bridge_over_a_gap_by_their_nearest_river` | 100 |
  | 8 | `vegetation::a_bare_biome_that_grows_on_its_own_surface_is_caught` | 98 |
  | 9 | `water::version_3_water_has_no_air_in_it_and_kelp_leaves_the_surface_whole` | 83 |
  | 10 | `vegetation::terra_pines_find_soil_on_the_cold_biomes_snow` | 79 |
  | 11 | `server::container::tests::loot::a_wilderness_chest_holds_only_its_biomes_table` | 66 |
  | 12 | `structures::meadow_stands_its_first_footbridges_over_its_version_3_rivers` | 63 |
  | 13 | `server::fluids::plants::tests::reach::the_shared_spreaders_keep_a_generated_patch_where_worldgen_put_it` | 58 |
  | 14 | `server::lod::tests::seams::no_seam_wall_the_finer_ground_leaves_open_is_dropped` | 53 |
  | 15 | `server::lod::tests::levels_one_two_and_three_are_the_downsample_of_their_generated_children` | 42 |
  | 16 | `server::lod::tests::a_direct_probe_generates_its_levels_at_their_own_resolution_and_leaves_finer_ones` | 42 |
  | 17 | `caverns_under_the_water_table_are_underground_lakes_and_the_rest_are_air` (`tesserae-core`) | 41 |
  | 18 | `server::lod::tests::a_columns_probe_draws_one_height_per_column_with_no_caves_from_its_level` | 30 |
  | 19 | `server::fluids::plants::tests::reach::the_shared_spreaders_grow_a_patch_a_little_in_an_hour_and_never_fill_the_ground` | 30 |
  | 20 | `food::a_new_world_has_food_and_wood_within_a_short_walk_of_the_spawn` | 30 |

- CPU seconds (user + system) of the test binary built before and after, each test run alone with `--exact`, a before and its after side by side so they share the load (10 to 60). Settlements and templates 5998 + 981 to 2095; the census's three tests 296 + 334 + 250 to 83; the two density tests 248 + 274 to 65; Wood at 0.5 m 297 to 51; fields 698 to 377; food within a walk 810 to 85; food and wood at 1 m 86 to 20; oaks, apples and wheat 133 to 9; generated apples 18 to 5; bare Biome 109 to 30; water 117 to 30; bridges 995 to 285; footbridges 650 to 210; the barrow's chest 591 to 117, and the three village-chest tests 50 to 11 each; ground cover 245 to 176. The settlement figure varies most: it generates on 48 threads, which spin harder the idler the host is.
- Where the time went: every slow test generates Chunks, and version 3's Chunk is mostly drainage and noise for the column, then Plants. Its band at 1 m is about 15 Chunk rows, of which a column's ground and what stands on it fill two or three; the sky Chunks above the ground but under the band's top cost the most, since each still places its neighbours' Plants. So the cuts are fewer Chunks, by four means.
- One pass where there were several, nothing asserted less. Wood at 0.5 m scans once for Wood and oak logs, not once each. The oaks, apples and wheat test scans once for its seven Materials, not seven times, and stops when each group is found. The food tests stop at the first food, and at 1 m once 15 Wood is in sight too: they assert only that much is there. The scan goes nearest Chunk column first.
- Generators shared across cases, by folding tests that generated the same Chunks into one, each assertion kept word for word. `jungle_and_swamp_trees_hang_vines` and `generated_wood_is_only_ever_the_trunk_of_a_grown_tree` run on `every_biome_of_every_demo_game_grows_its_declared_species_and_none_is_bare_by_accident`'s census. `jungle_and_swamp_are_denser_than_savanna_prairie_and_desert_in_every_game_that_has_both` runs on `each_biome_scatters_its_trees_at_its_density_class`'s densities, still counting five Games compared. `a_world_of_each_game_lays_small_and_large_settlements` runs in `a_world_of_each_game_shows_its_templates_and_a_settlement_level_whole_and_furnished` on its 768 m scan, out of which the 320 m scan is cut, with an assert that every one of its Chunk columns is there.
- Rows, named and justified:
  - `Generated::around` (Settlements and templates, bridges, footbridges, every loot chest scan), fields and the generated apples keep, in each Chunk column, the rows within `SETTLED_BAND_M` (24 m below, 40 m above) of its lowest and highest ground, not the whole band. That band is the suite's own reach for a Structure on its ground (`around_ground` already used it round the town); the tallest Template is 23 m with 4 ground layers, a crop field and its ditch lie on the ground, and an apple hangs on a tree standing on it. The bridge, footbridge, barrow and chest asserts are of something standing, which a narrower scan can only fail to find.
  - The census, the densities and ground cover skip each Chunk column's rows above a ceiling no Plant reaches: the highest ground of the column and its eight neighbours (no crown reaches further, asserted), plus `landforms.overhang_m`, the tallest Plant's `rise_m` and 8 Voxels of rounding, over the generator's own 3.
  - The census and the densities skip rows wholly below the column's lowest ground. The census counted only what stands on the surface already, so for it nothing changes. The vine and Wood checks read that surface census now, not the every-row one; on this sample the two held the same vine, log and loose-Wood counts in all five Games, and every Game's densities were identical with and without those rows. Ground cover keeps them: its measure counts cave mushrooms under a block.
  - Wood at 0.5 m keeps the rows within 48 m of the spawn's height, the rule `a_new_world_has_food_and_wood_within_a_short_walk_of_the_spawn` already used ("which a short walk keeps to"); so do the existence scans that share the helper. Its loose-Wood check no longer reaches caves more than 48 m under the spawn; the census still looks for loose Wood over every Game's surface.
- One sample reduced: `version_3_water_has_no_air_in_it_and_kelp_leaves_the_surface_whole` takes one column in four of its grid (118 by 122 m apart, the same 12 km square), still 1803 wet and 53 with kelp against its thresholds of 200 and 20.
- `.config/nextest.toml` schedules the merged Settlement test first, then the next nine longest.
- No generation code changed (only `crates/tesserae`'s tests and the nextest config), so no recorded digest moved.
- CI under 20 minutes is not ticked: it is checked on the next push. By the before run's ratio (66 min on CI for 18376 s here) the run would be about 17 min and the job about 22; ground cover, fields and the merged Settlement test are the longest left.
- CI at 8cb72a82 (run 44412788175): `[2515.015s] 3020 tests run: 3020 passed (22 slow), 8 skipped`, the job 48 min (5 min building). The projection above was wrong: the summed test time here was taken under shifting load, and CI's 4 threads did 9996 s of work. Six tests were half of it, and the longest alone ran 21 min, past the target before any other: Settlements and templates 1256 s, fields 1047, Wood at 0.5 m 899, ground cover 813, the densities 639, the census 385.
- Each of those six is now one test per Game (the Wood test one per seed), named `<check>::<game>` by `each_case!` in `demo_games.rs`; every assertion is the same, run on one Game. The densities' `assert_eq!(compared, 5)` is each Game asserting it has dense and sparse Biomes to compare. Locally, under load 46, the 31 cases pass and the longest is fields in Terra at 267 s.
- CI's Test job runs as four shards (`--partition slice:k/4`, doctests on the first), and a `Test` job passes only when all four do, so the required check keeps its name. Estimate: 1 min setup, 5 min build, about 10.5 min of tests per shard.
- CI at 1460f2d7: the four shards ran 13.7 to 13.9 min side by side, and `Test` passed 14.1 min after the run started.
