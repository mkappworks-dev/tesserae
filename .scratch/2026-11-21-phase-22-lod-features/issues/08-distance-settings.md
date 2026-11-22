# 08: The Distance settings tab

Status: done
Blocked by: 07

Spec: [LOD features](../spec.md).

## What to build

A Player sets how far and how sharp distant terrain is from Settings. A Distance
tab after Display, as approved in the mockup: a Preset dropdown (Low, Medium, High,
Custom; Medium the default) with its description, Reach and Detail sliders that only move
under Custom, Custom remembering its own values, and a table of each level's
Voxel size, ring, Chunk count and estimated memory that follows the sliders.
The table and the sliders' ends use the World's own view distance and ceiling,
not the mockup's 160 m and 8 km. Detail is shown in whole Chunks and sent as metres (`u` = Chunks ×
32). Low is the Game's own rings: its view distance, and the reach it declared before this step. 07's
`QualityCurve::balanced` and `detailed` and the `TESSERAE_QUALITY` names become `low`, `medium` and
`high` (spec Q24). Applying sends the curve to the Server (07) and
keeps it in the Player's profile.

## Acceptance criteria

- [x] Settings tests for the dropdown, the locked sliders, Custom's memory,
      apply, discard and restore defaults.
- [x] Applying a preset changes the rings the Server streams.
- [x] A capture of the tab, shown to you beside the mockup.

## Comments

- The preference is `client::quality::Distance` in the profile (`distance`, a
  preset and Custom's own Chunks and km; absent reads as Medium). Ticket 12
  made Low the default again, so absent reads as Low, until 09 passes its
  budgets. _Superseded by 09: Medium is the default again, so absent reads as
  Medium._ The Client
  sends its curve when a World opens and whenever an applied change asks for
  another; the Server's `quality::apply` clamps it as in 07. The Client only
  reads `DeclaredStreaming` and `WorldSettings`, for the sliders' ends and the
  table.
- Presets are `QualityCurve::low(game)` (the Game's `view_distance_m` and
  `lod_distance_m`), `MEDIUM` (256 m, 6 km) and `HIGH` (384 m, 8 km, the
  mockup's 8 rather than 8192 m). _Superseded by 09: `MEDIUM` is 192 m (6
  Chunks) to 6 km._ `TESSERAE_QUALITY=low|medium|high|<unit_m>,<reach_m>`
  overrides the preference in `dev` and `profiling`.
- Detail counts 32 m Chunks whatever the World's Voxel size, so a Custom curve
  is the same metres in every World. Custom's values are stored unclamped; the
  sliders show what the World streams (Terra: Reach 2 to 3 km, Detail 5 to 12).
  As in the mockup, moving a slider onto a preset's values chooses that preset
  and leaves Custom's memory as it was.
- The table counts the Server's own mesh shells (`LevelRings`, holes excluded),
  so its Chunks and MB sit below the mockup's full discs: Medium on Terra is 4
  coarse levels to 3 km, 4,808 Chunks, 35 MB at the mockup's 52 KB per column.
  _Superseded by 09: under Terra's 8192 m ceiling, Medium is 5 coarse levels
  to 6 km._
- For 09: Terra's ceiling rising to 8192 m makes Low reach 8 km, as Low reads
  the Game's `lod_distance_m`. Give Low its own reach (the Game's pre-step-22
  value: 3000 m on Terra, 4096 m by default) in `QualityCurve::low`, the one
  place it is decided, e.g. from a new field of `DeclaredStreaming`.
- Tests: `export RUSTFLAGS="-D warnings"; cargo test --workspace --locked --features tesserae/dev --no-fail-fast`
  — 3096 passed, 0 failed. A first run failed
  `an_unchanged_wet_chunk_leaves_mid_rescan_without_a_commit_and_settles_on_return`
  at load average 21; it passed three times alone and in the rerun. New:
  `client::quality::tests` (6), `client::settings::tests::the_distance_*`,
  `a_distance_preset_waits_for_apply_and_discard_and_restore_defaults_undo_it`,
  `applying_a_preset_changes_the_rings_the_server_streams`, and
  `lod_rings::tests::{low_is_the_game_s_own_doubling, medium_reaches_eight_of_each_level_s_own_chunks, high_holds_each_level_twice_as_far}`.
  Clippy (`--all-targets --all-features`), docs, the default-feature check,
  rustfmt and Prettier are clean.
- After merging `feat/phase-22-lod-features` at ea48f268 (ticket 03): 3102
  passed, 1 failed. 03's `level_one_cells` now reads `QualityCurve::HIGH.unit_m`
  for the old `DETAILED_UNIT_M` (both 384 m).
  `server::lod::tests::a_level_one_job_counts_the_level_zero_chunks_it_generates_inside_level_zeros_load`
  fails the same way (`in_load: 203`, expected 0) at ea48f268 without this
  ticket, so it is 03's to fix.
- Capture (`TESSERAE_SCREENSHOT_OFFSCREEN=1`, `TESSERAE_SCREENSHOT_SETTINGS_TAB=distance`,
  Terra, 1440×1100): `/tmp/lod22-t08-distance-CMh5` — `engine-distance.png`
  beside `mockup-distance.png`. An offscreen capture now draws the UI for an
  open Settings as for the picker menu. "Unapplied changes" shows in every
  Settings capture (Display too, `engine-display.png`), not only this tab.

