# 04: Sun and moon light

Status: ready-for-agent
Blocked by: 02

Spec: [Day and night cycle](../spec.md).
[ADR-0039](../../../docs/adr/0039-the-sky-is-our-own-shader-not-bevys-atmosphere.md).

## What to build

The light moves through the day and night on Terra, from the Sky state.

- `sky` gains the sun's tilt (toward the south, +Z), validated in [0, 60];
  Terra declares 25°.
- The Sky state, one pure function in `tesserae-core`: this ticket fills the
  sun and moon directions, the light, the ambient, the sky and fog colours and
  the Moon phase from the engine's default Day curve.
- The Server's sky message gains a flag for a Cave Biome's air; the Client
  keeps the eased pair in its own resource and writes the Sky state's colours
  to `FogSettings` and the clear colour. Cave air is never tinted.
- One shadowed `DirectionalLight`: the sun while it is up, the moon while it is
  down, its strength times the horizon fade whatever the curve says, so zero
  where it swaps. Moonlight scales with the Moon phase.
- The sky's sun disc reads the sun's direction from the Sky state, not from the
  light, and is not drawn below the horizon.
- The ambient dome dims at night; a face that sees no sky keeps the noon floor
  from its own uniform, so a sealed cave is the same at every hour. Block light
  is unchanged.
- Underwater fog and the clear colour behind it dim and tint with the ambient;
  an emitting fluid keeps its own.
- `DistanceFog` gains its glow toward the light, from the light's colour.

## Acceptance

- [ ] Sky state tests: sunrise and sunset at 06:00 and 18:00, the noon sun the
      tilt toward the south, zero light on both sides of the swap whatever the
      curve, full moon brighter than new, cave air untinted, no `sky` is
      today's noon.
- [ ] One shadow caster still lights every level (the one-sun test holds).
- [ ] Captures at the baseline Eyes at noon, dusk, full-moon and new-moon
      midnight, approved by the user; the `cave` fixture's captures at noon and
      midnight match; the map at midnight is the map at noon.
