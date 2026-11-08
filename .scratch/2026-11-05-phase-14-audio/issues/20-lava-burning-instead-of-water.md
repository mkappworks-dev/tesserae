# 20: Lava burning instead of water

Status: ready-for-human

## What to build

Lava currently shares all fluid water cues. Select fluid feedback from declared
heat, not Material names. Emit Burn on hot entry, Splash on cold entry/exit,
and nothing on hot exit; observe direct transitions and avoid per-tick repeats.
Do not play water movement cues or underwater beds in hot fluid. Append the
Game-bindable burn Cause to unreleased 0.3, including data Mods and starter WIT.
Terra supplies CC0 burning audio and a separate lava Material sound set.

## Acceptance criteria

- [x] Cold entry/exit and hot entry/exit/direct transitions have regression coverage.
- [x] Lava uses burning audio; water retains Splash; hot fluid has no water movement/bed.
- [x] Burn binds through component and data declarations; prior contract lines still load.
- [x] Relevant tests, lint, Game builds and Terra validation pass.
- [ ] User accepts lava and water entry audio in game.

## Comments

- The Server classifies contact from FluidDef heat. Cold entry/exit emits
  Splash; hot entry emits Burn once; hot exit is quiet. Direct cold/hot
  transitions work and initial contact remains silent. Missing Players reset
  contact. Hot fluid cannot emit Swimming/Floating or select water ambience.
- Burn is appended to unreleased 0.3 and the starter WIT, resolves through
  component and data Game declarations, and uses the Vitals detail trim.
  Earlier contract-line adaptation is unchanged and tested.
- Terra binds Burn to a 2.87 s CC0 fire recording from AntumDeluge at natural
  pitch and supplies a lava Material set, replacing water interaction sounds
  there too. The mono 44.1 kHz Vorbis file decodes and occupies 24,763 bytes.
- Full workspace suite: 2,058 passed, four existing ignored. Clippy with
  warnings denied, formatting, Game builds and Terra validation pass (build
  `6a8bb7cf6858`). Regression coverage includes fluid transitions, actual hot
  contact events, hot-fluid soundscape exclusion, component/data bindings,
  and the Vitals trim.
- In-game listening remains open. No commit or push. Repackage the demo for
  the new SoundCause and Terra recording before running it.
