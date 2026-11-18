# 19: Terra's field plots grow all its crops

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

The fields test compares against a World that drops only the field plots, so it
measures field plots alone (ticket 18's Comments). Meadow, Tidewater, Lumen and
Ember pass that way; Terra does not: near spawn at seed 7 its fields hold one
wheat and one potato plot, no carrot and no ditch, because it plants fields only
in its temperate Biome, every 72 m or 112 m, with carrot 2 in 10. Retune Terra's
fields (spacing, Biomes, crop weights) so its field plots alone carry the test,
and switch the test to the fields-only comparison.

## Acceptance criteria

- [ ] The fields test compares against a World without field plots only, and
      passes for every Game with its thresholds unchanged.
- [ ] Only Terra's content and that test change; no Engine code, and no other
      Game test breaks.
- [ ] What changed in Terra's fields, and the counts before and after, are noted
      in a comment here.
