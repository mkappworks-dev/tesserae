# 08: Soundscapes by time of day

Status: ready-for-agent
Blocked by: 02

Spec: [Day and night cycle](../spec.md).
[ADR-0025](../../../docs/adr/0025-sounds-are-declared-data-the-server-names-and-the-client-plays.md).

## What to build

Birds by day, crickets by night, blended at dawn and dusk.

- `ambient-bed` gains `when` (day, night, always by default) and
  `soundscape-def` gains `night-music`, at most 8 Sounds. The data-only TOML,
  the starter's contract and the compatibility doc change alongside.
- The Client scales day beds by Daylight and night beds by one minus it. A
  track starts from `music` when Daylight is at least 0.5, from `night-music`
  otherwise, and is never cut by the hour.
- Terra's outdoor Soundscapes (default, Temperate, Dry, Savanna, Cold) each
  mark a day bed and a night bed, CC0, under the same licence checks as its
  other sounds; Biomes may share a pair.

## Acceptance

- [x] Bed volumes by `when` across dawn and dusk, and track choice by the 0.5
      threshold, in `tesserae-core`'s audio tests.
- [x] A Soundscape with no `when` and no `night-music` mixes as today.
- [ ] Terra's day and night beds heard in the app by the user, in Temperate and
      one other Biome.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), and nothing listened to in the app, so the boxes stay unticked.

- Contract: `ambient-bed.when: option<bed-hours>` (`day`, `night`, `always`;
  `none` is `always`) and `soundscape-def.night-music: option<list<string>>`,
  both last. Data-only `when = "day"` on a bed and `night-music = [..]` on a
  row. Errors name the field `night-music` (more than 8, an unknown name, a
  non-`music` Sound); a bad `when` names the bad variant, like other enums.
  `none` night music plays `music` at every hour; an empty list is silence by
  night.
- Core, `tesserae_core::audio`: `BedHours { Day, Night, Always (default) }`
  with `gain(daylight)` (Daylight, one minus it, 1; Daylight clamped), on
  `Bed::when`; `SoundscapeDef::night_music: Option<Vec<SoundId>>`;
  `NIGHT_MUSIC_BELOW = 0.5` and `tracks_at(music, night, daylight)`. `Music`
  gains `change_by_hour(tracks, night, silence)` (`change` is it with `None`)
  and `set_daylight`, read only when a track starts, so a playing one is never
  cut. A start that finds no track for the hour waits another silence and
  looks again.
- Client (`client/soundscape.rs`): Daylight from
  `ClientTimeOfDay::daylight()` (its `expect(dead_code)` is gone), so a Game
  with no `sky` is noon: day beds whole, night beds silent, as today. A bed
  sink plays on at zero gain while its hours are out, so dawn fades it in
  rather than restarting it. The Server is unchanged; it names the Soundscape.
- Terra: every outdoor Soundscape keeps its wind and marks a day and a night
  bed. Default and Temperate share wind (always), birds (day, 0.8) and
  crickets (night, 0.7); Dry, which Savanna also names, wind (always),
  cicadas (day, 0.6) and other crickets (night, 0.7); Cold its wind by day
  and a darker wind by night. Underground and underwater are unchanged.
- Sounds, all CC0 1.0 from OpenGameArt, in `games/terra/CREDITS.md`:
  `birds.ogg` (Thimras, park-ambiences), `crickets.ogg` (syncopika,
  cricket-sounds), `cicadas.ogg` (syncopika, cicada-sounds), `dry-night.ogg`
  (Wolfgang_, crickets-ambient-noise-loopable), `cold-night.ogg`
  (Luke.RUSTLTD, wind1, `wind3.wav`). Each is filtered, loudness-matched,
  given an overlapping loop seam and encoded with oggenc -q 3, as before.
- Decided: the CREDITS note reserving bird calls for future animals now
  reserves positional calls only; distant birdsong as a day bed is what the
  spec asks for. Terra declares no `night-music`; it has one track.
- Tests: `tesserae-core` `tests/audio.rs` (beds by `when` through dawn and
  dusk; no `when` is always) and `tests/music.rs` (the 0.5 threshold, no
  night music is as today, each start reads its hour, never cut, empty night
  music, night music alone); `tesserae-mods` `tests/soundscapes.rs` and
  `tests/data.rs` (resolving, each rule's field, the `full.toml` fixture);
  Client `client/soundscape.rs` (noon and midnight beds and track, a day
  track plays on into the night); Terra's pairs in `demo_games.rs`.

The tests pass in the full `scripts/cargo-ci` run of 2026-11-26.
