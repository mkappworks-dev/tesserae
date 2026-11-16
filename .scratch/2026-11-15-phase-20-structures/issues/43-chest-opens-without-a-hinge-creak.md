# 43: The chest opens without a hinge creak

Status: done

Spec: [Phase 20](../spec.md).

## What to build

Play-testing: "update the chest opening sound. i do not like the current hinge
sound." Terra's `container-open.ogg` was a filtered creak. It becomes a latch
click followed by a wooden lid lift, from the same CC0 Kenney RPG pack, at the
same -8 dBFS peak and about the same length.

## Acceptance criteria

- [x] `games/terra/audio/container-open.ogg` replaced; no creak.
- [x] `games/terra/CREDITS.md` names the new sources.

## Comments

`metalClick.ogg` from 45 ms, high-passed at 300 Hz, then `bookOpen.ogg` 95 ms
later, pitched to 0.82, band-limited 90–6000 Hz; mono 44.1 kHz, peak -8 dB, 0.38
s, oggenc q6.
