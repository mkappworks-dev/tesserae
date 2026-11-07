# 09: Terra is heard

Status: ready-for-agent

Blocked by: 03, 05, 06, 07, 08, 11

## What to build

The Terra Game ships sound as the reference: Sound sets for its Materials, a
Sound for every non-Voxel Cause, Soundscapes for the open world, underground
and underwater, and a Soundscape for each of its Biomes that should sound
different. The other demo Games stay silent.

- Recordings are CC0 only (Kenney, or OpenGameArt and freesound entries that
  are CC0), converted to Ogg Vorbis with local ffmpeg, mono for Effects, about
  2 MiB in all. No CC-BY, no non-commercial licence, no Sonniss bundle.
- A `CREDITS.md` in Terra's folder lists every recording with its source URL
  and licence.
- `architecture.md` describes the audio path; the DESIGN.md roadmap row for
  step 14 records what was done.

## Acceptance criteria

- [ ] Every Terra Material a Player meets has a Sound set; every Cause and
      context is bound.
- [ ] Every recording is CC0, listed in `CREDITS.md` with its URL.
- [ ] Recordings total about 2 MiB and pass `tesserae validate`.
- [ ] The other demo Games and their Worlds open unchanged and silent.
- [ ] The user has listened to a Terra World, in Creative and Survival, and
      accepted it.
