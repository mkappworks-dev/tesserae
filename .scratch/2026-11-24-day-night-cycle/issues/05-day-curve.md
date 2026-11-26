# 05: Day curve

Status: ready-for-agent
Blocked by: 04

Spec: [Day and night cycle](../spec.md).

## What to build

A Game declares its Day curve and Terra declares its own, so dawn, noon, dusk
and night look the way the Game wants.

- `sky` gains the Day curve: keyframes at hours, each with the sun's and moon's
  colour and strength, the ambient's colour and strength, a sky tint and a fog
  tint (a colour and an amount) and star brightness. Eased linearly, wrapping
  at midnight. The data-only TOML, the starter's contract and the
  compatibility doc change alongside.
- A tint moves the Biome colour toward the tint colour by the amount, in linear
  light.
- Terra declares a first curve: warm at dawn and dusk, blue at night, about 15
  to 20% of noon at full moon on open ground and about 8% at new moon.

## Acceptance

- [ ] Each rule (1 to 16 keys, hours ascending in [0, 24), strengths, amounts
      and colours in [0, 1], ambient at every key at least the cave floor) is a
      load error naming the field.
- [ ] A key with amount 0 returns a Biome pair unchanged; a `sky` without a
      curve uses the engine's default.
- [ ] Terra's curve approved by the user on captures at noon, dusk, full-moon
      and new-moon midnight.
