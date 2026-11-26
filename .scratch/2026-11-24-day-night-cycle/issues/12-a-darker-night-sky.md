# 12: A darker night sky

Status: ready-for-agent

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

- [ ] Captures at full-moon and new-moon midnight and 03:00, far and near Eyes,
      approved by the user.
- [ ] The reach's edge stays hidden at night.

## Comments

Recommended on 2026-11-26 and agreed by the user.
