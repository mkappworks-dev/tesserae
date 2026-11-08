# 09: Terra is heard

Status: ready-for-human

Blocked by: 03, 05, 06, 07, 08, 11

## What to build

The Terra Game ships sound as the reference: Sound sets for its Materials, a
Sound for every non-Voxel Cause, Soundscapes for the open world, underground
and underwater, and a Soundscape for each of its Biomes that should sound
different. The other demo Games stay silent.

- Recordings are CC0 only (Kenney, or OpenGameArt and freesound entries that
  are CC0), converted to Ogg Vorbis with local ffmpeg, mono for Effects, within
  the 32 MiB referenced World budget. No CC-BY, no non-commercial licence,
  no Sonniss bundle.
- A `CREDITS.md` in Terra's folder lists every recording with its source URL
  and licence.
- `architecture.md` describes the audio path; the DESIGN.md roadmap row for
  step 14 records what was done.

## Acceptance criteria

- [x] Every Terra Material a Player meets has a Sound set; every Cause and
      context is bound.
- [x] Every recording is CC0, listed in `CREDITS.md` with its URL.
- [x] Longer beds and full tracks fit the 32 MiB referenced World budget and
      pass `tesserae validate`.
- [x] The other demo Games and their Worlds open unchanged and silent.
- [ ] The user has listened to a Terra World, in Creative and Survival, and
      accepted it.

## Comments

- The initial implementation embedded CC0 Ogg Vorbis recordings (about 1.4 MiB), with every source and
  conversion listed in `games/terra/CREDITS.md`. No other Game binds Sounds.
- Headless validation passed with `cargo run -p tesserae --offline -- validate
  games/terra --mods mods --trust-for-run`.
- The final Creative and Survival listening check remains for the user; this
  ticket stays open until that acceptance.

- The comfort pass supersedes the original 2 MiB target with longer beds and full music tracks, under the existing 32 MiB World budget. Final automated validation passes; listening acceptance remains open in this ticket and the subsequent feedback tickets.
