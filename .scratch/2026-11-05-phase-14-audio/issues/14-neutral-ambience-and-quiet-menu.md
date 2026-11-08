# 14: Neutral ambience and a quiet Take a moment menu

Status: done

## What to build

User reports an annoying tonal “ooo” and glitches behind Take a moment. Birds
should be reserved for nearby animals when those exist. Remove bird-containing
background recordings from Terra. Silence World playback smoothly while Menu,
Settings or a save modal is open; retain music/bed position and do not replay
queued action sounds on resume. Do not change authoritative simulation.

## Acceptance criteria

- [x] Terra's background beds and music no longer embed bird calls.
- [x] Identify and remove the unwanted tonal source, with user confirmation if needed.
- [x] Menu/Settings suppress new effects, clear old one-shots after a short fade,
      pause bed/music sinks and their scheduler, and resume cleanly.
- [x] Tests cover menu entry, queued events, scheduler freeze and clean resume.
- [x] Game builds, tests, lint and validation pass; no commit or push.

## Comments

- User confirmed “ooo” remains with Music muted and occurs in green areas.
  Replaced the temperate bed with 104.59 s of field-recorded wind/rustle,
  filtered and seam-edited. Removed the bird-and-synth music track from Terra's
  declarations and selection; it remains on disk only as a retired reference.
- Take a moment, Settings and save modals fade World playback out over 0.1 s
  using real time, pause background sinks and music scheduling, and discard
  one-shots. They do not change authoritative simulation.
- Music slider regression follows Navigate Audio → slider change → Apply →
  resume, checks 0 and 100 on the same existing track, and verifies Ambient
  remains independent. Settings now explains Apply and resume.
- Engine suite: 1,165 passed, one existing ignored. Demo Games rebuilt; Clippy,
  formatting and Terra validation pass. No commit or push.
- Subjective listening remains open under tickets 09 and 13; the user must
  repackage the demo to hear the changed assets.
