# 04: Volume settings

Status: ready-for-agent

Blocked by: 02

## What to build

A player's Master, Effects, Ambient and Music volumes, from 0 to 100, are saved
in the user profile beside view mode, default to 100, 100, 70 and 50, and apply
to every sound, including those already playing. The Settings page that sets them is ticket 10.

## Acceptance criteria

- [ ] The four values round-trip through the profile; a profile without them
      loads the defaults.
- [ ] Core gain is Master × Sound category × Sound volume × attenuation, each
      setting mapped from 0–100; tested at 0, 50 and 100.
- [ ] Changing a value recomputes every playing sink's volume the same frame;
      Bevy's `GlobalVolume` stays at 1.
- [ ] Volumes are a user preference, never World state or saved with a World.
- [ ] Tests at the core audio and profile seams.
