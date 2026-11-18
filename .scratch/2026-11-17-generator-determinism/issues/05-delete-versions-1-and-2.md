# 05: Delete Generator versions 1 and 2

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 02, 03, 04, 15, 16, 17, 18.

## What to build

Version 3 is the only Generator version (ADR-0033). Versions 1 and 2's
generation, every version gate, their digests and tests, and the version 2
fixture parameters are deleted; the fixtures' default is version 3. The save
supports generator version 3 only: a World stamped 1 or 2 is refused with a
message naming its version and saying it predates version 3. The reference docs
that describe versions 1 and 2 (glossary, world persistence, data mods, contract
compatibility, the design document's step notes) are corrected in the same
change.

## Acceptance criteria

- [x] No code path asks which Generator version a World uses, except the save's
      check.
- [x] A save test shows a World stamped 1 or 2 is refused with that message.
- [x] The version 3 digests are unchanged.
- [x] No doc outside ADRs and `.scratch/` describes version 1 or 2 as
      generating.
- [x] No test is pinned to version 1 or 2. The one exception,
      `each_biome_scatters_its_trees_at_its_density_class`, runs under version 3
      ignored with a reason naming ticket 13.

## Comments

- Deleted: versions 1 and 2's heightfield (the one-fBm `surface_height_m` path
  and its `inv_feature_size`), the version 2 clamp of the lake level to the
  band, the `BIOMES_SINCE` and `LANDFORMS_SINCE` constants and every gate on
  them (worldgen, Structures, Settlements, the save's snapshot and restore),
  `SUPPORTED_GENERATORS`, and the `Option` round version 3's `Terrain` with the
  `is_none` branches that stood for "version 1 or 2" (`level_disc`, plant
  crowding, a Plant's water cover, `dry_land_near_m`, `surface_top_m`,
  `climate_at_height`). `landform_fields` returns its fields and the Climate's
  Biome choice takes a site, no longer an `Option`.
- The save refuses a World stamped 1 or 2 with "generator version N predates
  version 3, the only one this Engine generates", and any other non-3 as before.
  `WorldgenParams::validate` still refuses a generator other than 3; with the
  save's check these are the only reads of the number. `restore` keeps its
  `generator` argument, which it copies into the parameters and never branches
  on.
- `testgen::world_params()` is now version 3's fixture (the former
  `world_params_v3`); the version 2 fixture is gone. `generator_version.rs`
  keeps its recorded version 3 digests on the fixture ground without Biomes, as
  recorded, and its coverage test now finds caves in the high-ground box: under
  version 3 the origin box sits at or under Sea level at every seed, where no
  tunnel opens.
- Tests deleted, each of version 1 or 2 itself: `existing_seeds_keep_their_terrain`,
  `version_1_ignores_cave_tuning`, `water_never_rises_above_the_highest_ground`,
  the version 1 and 2 save tests listed in 02 (the "older Worlds save none"
  halves trimmed, the three tests renamed), `generator_version_1_ignores_*` in
  `biome_ground`, `biomes` and `ore_veins`, `version_2_*` in `plant_worldgen`,
  `strata`, `underground` and `terrain_v3`, `earlier_versions_spawn_where_they_always_did`,
  the version 2 digests of `coarse_generation`, `settlement_worldgen`,
  `surface_features` and `generator_version`, the version 2 halves of
  `terrain_biomes` and `terrain_v3`, `sky_fog`'s before-Biomes test, the
  Server's two version 1 reopen tests and the dev override test. The save's
  Writer itself refuses a commit stamped 1 or 2, so the Server cannot stage one
  without a hashing dependency; `save::tests` covers the refusal.
  `the_facts_carry_the_games_climate_sizes_and_the_worlds_generator` carries 3.
- `each_biome_scatters_its_trees_at_its_density_class` runs under version 3,
  ignored, naming ticket 13.
- Also deleted: the dev-only `TESSERAE_GENERATOR` override and
  `scripts/profile-frames --generator`, which could only pick 3; and
  `TESSERAE_WATER_LEVEL_M`, since version 3 reads no level, replaced by
  `TESSERAE_WATER=0` beside the caves and smoothing switches.
- Benches: `structures`' valleys and `coarse_breakdown` no longer set a lake
  level. Ticket 02 said version 3 ignores `relief_m` and `feature_size_m` too;
  it does not, as its detail octaves read both, so the benches keep them.
- Left, as contract or save: `WaterParams::level_m` is read by nothing (water
  stands at Sea level) but stays, since the WIT `water-params` record and the
  save's `lake` carry it; `with_default_lake` sets it to Sea level. The 0.3 WIT
  doc comments on `biome-selectors` (`content.wit` lines 815 and 840, also in
  `starters/rust/wit/`) still say "version 3 only" and "a Biome declared before
  version 3 is unchanged"; the WIT text is unchanged here. The Games still
  declare a `level-m`, which nothing reads.
- Docs corrected: GLOSSARY.md (the Water level entry folded into Sea level,
  Generator version, Climate, Biome, Stratum, Aquifer), docs/world-persistence.md,
  docs/data-mods.md, docs/contract-compatibility.md, docs/authoring-guide.md
  (incl. `[world.water]`'s `level-m`), and DESIGN.md's step 17 note.
- Version 3 digests unchanged: every recorded digest test in tesserae-core
  passed before and after (`generator_version`, `coarse_generation`,
  `surface_features`, `settlement_worldgen`).
- Failing before and after this change, at 6e549c07 too, with
  `--features tesserae/dev`:
  `server::gallery::tests::parts_follow_properties_and_missing_kinds_are_omitted`
  (the sparse fixture's `fence` is `Some(14)`) and
  `server::streaming::tests::every_mesh_carries_the_sight_lines_of_its_chunk_as_it_stands`.
  Not investigated here.
