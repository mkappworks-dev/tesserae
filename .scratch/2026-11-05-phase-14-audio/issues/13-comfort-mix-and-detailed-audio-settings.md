# 13: Comfortable ambience and detailed Audio settings

Status: ready-for-human

## What to build

User reports unpleasant short repeating audio and requests long recordings,
research-backed balancing, per-action volume controls and an updated mockup.
Read [research](../audio-comfort-research.md) and review
[the interactive mockup](../audio-settings-comfort-mockup.html). The menu
prototype's Audio page is updated to the same proposed controls. The original
user instruction requires agreement on this UI before Engine implementation.

## Acceptance criteria

- [x] Audit durations, source loudness and constraints; cite primary research.
- [x] Update mockup with main mix and expandable per-action controls.
- [x] User agrees the revised settings layout.
- [x] Ship long, verified CC0 recordings with edited seams and credits; do not
      pad short loops to claim longer duration.
- [ ] Quiet varied beds and non-repeating full music tracks pass listening checks.
- [x] Replace clockwork floating with sparse varied Server-driven water details.
- [x] Add a separate bounded Ambient asset limit; retain World accounting and
      check transient memory during loading/crossfades.
- [x] Persist per-action trims with old-profile defaults; apply to live Sounds;
      preserve draft, Apply, Discard and Audio-only reset behavior.
- [x] Test mixer/scheduling, loader bounds, migration and Settings seams.
- [ ] Measure and audition the busy-scene mix on headphones and speakers.

## Comments

- Research and mockup are complete; current Engine and packaged audio are
  unchanged by this proposal. No commit or push. Mono output is a useful later
  addition; avoid controls for voice, dialogue or UI sounds that do not exist.

- User approved this layout and proposal with “ok”; implementation authorized,
  with no commit or push. Final listening acceptance remains human-owned.

- Implemented and checked; see [validation](../comfort-validation.md) for test,
  asset and process-memory evidence and the remaining hardware/listening limits.
- The standards review found and the implementation fixed silent-music ducking.
  No unaddressed code defect was reported. No commit or push.
