# 16: Gentle surface swimming feedback

Status: done

## What to build

User hears a machine-gun-like loop when floating and moving forward. Moving at
the surface uses Swimming, not idle Floating. Distance-only triggers at 1.2 m
allow short splash clips to overlap as movement speed rises. Require a minimum
1.6 s between swimming sounds, with seeded 1.6–2.2 s spacing and roughly 3 m
travel per stroke. Preserve the cooldown through brief stops and wet/dry
transitions. Replace the sharp splash with quiet water strokes and variations.
Do not change movement speed, fluid physics or entry/exit splashes.

## Acceptance criteria

- [x] Swimming sounds cannot overlap the reference stroke clips at any speed.
- [x] Server tests cover normal/high speed, brief stops and re-entry, still
      floating, seed determinism, and dry/grounded silence.
- [x] Terra uses longer, softer water-stroke variants with credits.
- [x] Tests, lint, Game build and validation pass; no commit or push.

## Comments

- Swimming, including forward movement while surface-floating, now requires
  3 m travelled and a seeded 1.6–2.2 s cooldown. Distance saturates rather than
  queueing bursts, and stop/start or wet/dry transitions cannot reset the wait.
- Terra has three quiet water gestures lasting 1.14–1.2 s. At the minimum
  declared pitch their longest playback is 1.263 s, below the 1.6 s minimum
  event spacing. They replace the sharp splash and have gentler attacks/tails.
- Server regressions cover 3, 30 and 300 m/s, idle floating, seed replay and
  brief stops/re-entry. Movement and fluid physics are unchanged.
- Full Engine suite: 1,169 passed, one existing ignored. Clippy, formatting,
  demo Game builds and Terra validation pass. Listening remains under 09/13.
- No commit or push. Repackage the demo to replace the old sharp recording.
