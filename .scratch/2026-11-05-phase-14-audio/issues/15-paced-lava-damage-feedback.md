# 15: Pace lava damage feedback

Status: done

## What to build

User reports excessively fast, short death/damage sound and animation when
stepping into lava. Hurt audio currently emits every fixed damage tick and
visual feedback retriggers on each whole point lost. Limit feedback, not damage:
immediate hurt feedback, then at most one hurt sound per 0.75 s of continued
harm; one death event. Slow the injury flash/low-health pulses and extend the
Terra death recording's release. Keep health loss and death timing unchanged.

## Acceptance criteria

- [x] Continuous lava/contact damage has bounded hurt cadence; death occurs once.
- [x] Damage totals are unchanged; Creative and dead Players stay silent.
- [x] Visual feedback fades fully rather than restarting on every decrement;
      existing Reduced motion behavior is preserved.
- [x] Terra death recording has a longer, single release with credits updated.
- [x] Regression tests, lint, Game build and validation pass; no commit or push.

## Comments

- Server hurt feedback is immediate and then limited to once per 0.75 s, with
  unchanged per-tick damage. A death transition emits once; dead Players stay
  silent and the feedback interval resets for the next life.
- Injury flashes last 0.65 s without restarting on every health decrement.
  Low-health edge pulses take 3 s and tile pulses 1.5 s. Reduced motion remains
  still. This changes damage presentation, not lava lethality.
- Terra's death recording is 1.559 s, with a lower-pitched body, soft irregular
  reflections and a fading release. Playback pitch is fixed so it is not
  randomly sped up. Source credits retain Kenney's CC0 original.
- Full Engine run: 1,167 passed, one existing ignored. Extended contact/next-life
  regression passed separately. Clippy, formatting, demo builds and Terra
  validation pass (build b309aa6bcd98). Listening acceptance remains under 09/13.
- No commit or push. Repackage demo assets before listening.
