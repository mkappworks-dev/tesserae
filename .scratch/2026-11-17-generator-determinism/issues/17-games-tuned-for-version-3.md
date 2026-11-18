# 17: The Games are tuned for version 3

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

Seven Game tests ticket 04 left on version 2 (see its Comments) fail under
version 3 because the Games' data was tuned for version 2's land. Retune Terra,
Tidewater, Meadow and Ember (Biome climates and shares, tree spacing and fit,
bare Biomes, Settlement sizes and spacing, crop weights) so each test passes
under version 3 unchanged:

- Terra's temperate and Tidewater's lagoon Biomes cover the most ground.
- Terra's jungle trees stand 2.5 to 6 m apart and fit at least 95% of jungle
  columns.
- Meadow's dense Biomes are clearly denser than its sparse ones.
- Terra's alpine and beach Biomes are bare (no mushrooms, no neighbour's
  coconuts).
- Ember has a large Settlement within 768 m.
- Terra's wheat leads its fields.

## Acceptance criteria

- [ ] Every listed test runs under version 3 and passes with its thresholds
      unchanged.
- [ ] Only Game data changes, not Engine code; the Engine digests do not move.
- [ ] Captures of each retuned Game's spawn surroundings are taken for review
      (screenshots stay out of the repo) and listed in a comment here.
