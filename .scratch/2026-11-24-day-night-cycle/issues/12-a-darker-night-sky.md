# 12: A darker night sky

Status: done

Spec: [Day and night cycle](../spec.md).

## What to build

At full-moon midnight Terra's sky still reads light grey-blue, more dusk than
night. Deepen the night sky to a navy that leaves no doubt it is night, so the
stars and the moon stand out. Leave the ground's light as it is: about 12% of
noon at full moon and starlight at new moon are approved and keep the World
readable without torches.

- Terra's `night()` key: `sky_tint` and `fog_tint` darker or stronger; the moon
  glow and `MOON_WASH` in `sky.wesl` checked so a full moon still lights the sky
  round it.
- The reach's edge stays hidden at night: the fogged terrain meets the darker
  horizon with no line.

## Acceptance

- [x] Captures at full-moon and new-moon midnight and 03:00, far and near Eyes,
      approved by the user.
- [x] The reach's edge stays hidden at night.

## Comments

Recommended on 2026-11-26 and agreed by the user.

2026-11-27: the night key's sky tint is (0.002, 0.004, 0.024) at 0.97 and its
fog tint (0.004, 0.007, 0.026) at 0.96, a navy. The ground's light is
unchanged; `MOON_WASH` and the moon glow were left as they are, and no shader
changed, so frame time is as measured in ticket 09. Captured at full-moon and
new-moon midnight and full-moon 03:00 (far) and both midnights (near); the
fogged terrain meets the horizon with no line. The user approved the look on
2026-11-27.
