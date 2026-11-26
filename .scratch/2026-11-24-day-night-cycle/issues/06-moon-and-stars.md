# 06: Moon and stars in the sky

Status: ready-for-agent
Blocked by: 04

Spec: [Day and night cycle](../spec.md).
[ADR-0039](../../../docs/adr/0039-the-sky-is-our-own-shader-not-bevys-atmosphere.md).

## What to build

The night sky.

- A moon disc at the Sky state's moon direction, showing its Moon phase, not
  drawn below the horizon.
- Procedural stars from the view direction, rotating with the Time of day,
  fading in with the star brightness.

## Acceptance

- [ ] Captures at dusk and at full-moon and new-moon midnight, approved by the
      user; the moon's phase reads at a glance.
- [ ] The reach's edge stays hidden at every hour.
- [ ] GPU frame time of the sky on `flight` at noon and midnight, interleaved
      against `main`, at most 0.1 ms over it, recorded in Comments.
