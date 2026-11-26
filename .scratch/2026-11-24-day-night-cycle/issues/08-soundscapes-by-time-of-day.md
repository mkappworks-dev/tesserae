# 08: Soundscapes by time of day

Status: ready-for-agent
Blocked by: 02

Spec: [Day and night cycle](../spec.md).
[ADR-0025](../../../docs/adr/0025-sounds-are-declared-data-the-server-names-and-the-client-plays.md).

## What to build

Birds by day, crickets by night, blended at dawn and dusk.

- `ambient-bed` gains `when` (day, night, always by default) and
  `soundscape-def` gains `night-music`, at most 8 Sounds. The data-only TOML,
  the starter's contract and the compatibility doc change alongside.
- The Client scales day beds by Daylight and night beds by one minus it. A
  track starts from `music` when Daylight is at least 0.5, from `night-music`
  otherwise, and is never cut by the hour.
- Terra's outdoor Soundscapes (default, Temperate, Dry, Savanna, Cold) each
  mark a day bed and a night bed, CC0, under the same licence checks as its
  other sounds; Biomes may share a pair.

## Acceptance

- [ ] Bed volumes by `when` across dawn and dusk, and track choice by the 0.5
      threshold, in `tesserae-core`'s audio tests.
- [ ] A Soundscape with no `when` and no `night-music` mixes as today.
- [ ] Terra's day and night beds heard in the app by the user, in Temperate and
      one other Biome.
