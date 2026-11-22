# 12: Fill-time regressions from 04 and 08

Status: done
Blocked by: 04, 08

Spec: [LOD features](../spec.md).

## What to build

Ticket 11's captures showed holes at 10 s. A diagnosis (captures in
`/tmp/lod22-12-holes-hbyv`) found unfinished fill, not lost Chunks, from two
commits:

- **08 made Medium the default** before 09's budget gate. Medium holds 2.1
  times the Chunks of Terra's own rings at levels 1 to 3; the whole field fills
  in about 16 s against 9.5 s. Until 09 passes the budgets, the default preset
  is Low.
- **04 stands Structures and paths at every coarse level.** Level 5's Chunks
  went from 0.5 to 10 ms, the whole field fills 2.3 to 2.7 s later on Low, and
  level 0's fill went from 1.2 to 2.2 s, which the spec forbids. At 16 and 32 m
  a building is under one Voxel. Structures and paths stand at levels 1 to 3
  only, as bridges stand at level 1 only; level 4 and 5 columns keep the agreed
  ground.

If level 0's fill still regresses afterwards, find why coarse jobs delay it.

## Acceptance criteria

- [x] The default preset is Low; a profile without a saved choice reads as Low.
- [x] No Structure or path above level 3; tests say so.
- [x] On Low, from the diagnosis's Eyes: level 0 filled in about 1.2 s and the
      whole field in about 9.5 s, as at `ea48f268`, in Comments with the load
      average.

## Comments

2026-11-22: Done on `t3/lod-fill-regressions`.

- `DistancePreset::default()` is Low, and `Distance::default()` takes it, so
  a profile without a `distance` key reads as Low. Low's hint says "The
  default."; Medium's no longer does. The spec and ADR-0036 keep Medium as the
  default once 09 passes.
- `STRUCTURES_TO` is 3. `make_coarse` and `generate_persistent` pass no
  placement generator to `agree` (so no `pave`) and run no `stand_structures`
  above it; levels 4 and 5 keep the agreed ground. Over saved changes, levels 4
  and 5 show a placed or removed Voxel through the downsample's vote, as before
  04. ADR-0035's Impostors paragraph and the spec's Structure Impostors bullet
  record the reason.
- Level 0's fill came back with level 5's cost; no change to the job pool was
  needed. Every stage shares the three-thread async pool, and `LodBudget` lets
  16 coarse jobs queue ahead of a level-0 job: at 10 to 14 ms a level-5 job
  that queue held level 0 back by a second or more.

Tests: `server::lod::tests::structures::no_structure_stands_above_level_three`
(the tower World's level 4 and 5 Chunks stand no lantern; 194 at level 4
without the cap) and
`server::lod::agreement::levels_one_to_three_stand_terras_spawn_settlement_and_its_paths_as_level_zero_does`
(04's Settlement table at levels 1 to 3; at levels 4 and 5 every Chunk over the
window equals its Chunk made without the placement generator's Structures and
paths). `client::profile::tests::a_profile_without_a_distance_choice_reads_as_low`;
the quality and settings tests now start from Low.

Fill times. Release `dev` build, Terra, offscreen 1920 x 1080, no flight,
settle 15 s, the diagnosis's three Eyes (`at-2km` 1760.5, 300.5, 1264.5;
`join-1-2-forest` 1340.5, 105.5, -735.5; `join-2-3-forest` 1120.5, 135.5,
-735.5). `ea48f268` is the diagnosis's binary (the Game's own rings), "tip" is
`01d386f0`'s code with `TESSERAE_QUALITY=low`, "12" is this branch at its
default. Two rounds in mirrored order, host load 5.7 to 6.7. Level 0 is its
`filled_s`; the field is the last coarse level's; L5 gen is level 5's mean
job CPU:

| Eye             | build    | level 0 s  | whole field s | L5 gen ms  |
| --------------- | -------- | ---------- | ------------- | ---------- |
| at-2km          | ea48f268 | 1.55, 1.41 | 9.41, 9.27    | 0.77, 0.66 |
| at-2km          | tip, Low | 3.28, 3.34 | 12.71, 12.61  | 14.0, 13.7 |
| at-2km          | 12       | 1.52, 1.43 | 9.59, 9.54    | 2.22, 2.19 |
| join-1-2-forest | ea48f268 | 1.66, 1.66 | 10.38, 10.83  | 0.63, 0.66 |
| join-1-2-forest | tip, Low | 2.90, 3.03 | 13.49, 13.87  | 11.6, 12.5 |
| join-1-2-forest | 12       | 1.68, 1.59 | 10.66, 10.59  | 2.02, 2.03 |
| join-2-3-forest | ea48f268 | 1.47, 1.56 | 10.24, 10.20  | 0.67, 0.58 |
| join-2-3-forest | tip, Low | 2.41, 2.32 | 12.87, 13.98  | 11.3, 13.9 |
| join-2-3-forest | 12       | 1.58, 1.46 | 10.57, 10.26  | 1.99, 1.93 |

- Level 0 and the whole field are back within noise of `ea48f268` at every
  Eye; at this load both read some 0.3 s over the diagnosis's 1.2 and 9.5 s
  for `ea48f268` too.
- Level 5 jobs still cost about 2 ms against 0.65, and level 4's 0.65 against
  0.4, with no Structure drawn: most likely Canopy cover (05, 11), not
  profiled here. It does not move either fill time here; 09's series is where it
  would show.

Captures and logs (not committed): `/tmp/lod22-12-fill-kEjz`.

Ran: `cargo test --workspace --locked --features tesserae/dev --no-fail-fast`
(3,123 pass, 11 ignored), `cargo clippy --workspace --locked --all-targets
--all-features`, `cargo fmt --all`, `npm run fmt`.
