# 16: Every Game ships Templates and a Settlement

Status: ready-for-agent

Blocked by: 04, 05, 06, 07, 12, 13, 15

Spec: [Phase 20](../spec.md).

## What to build

Each of Terra, Tidewater, Ember, Lumen and Meadow declares fifteen Templates
from Materials Obtainable in it, as the approved mockup shows: ten Settlement
buildings (homes, an inn, a warehouse, a blacksmith, a bakery, a market stall, a
barn and a well) and five wilderness Structures. Each declares at least one
Settlement that uses them, and Terra also a town. Stations a building needs (an
anvil, a forge, an oven) are declared with the step 16 machinery. Each Game's
Biomes list them. The SDK starter declares one Template as the documented
example. The Engine ships none.

## Acceptance criteria

- [ ] A World of each Game shows its Templates and a Settlement, level, whole
      and furnished, with cellars underground.
- [ ] Every Game passes the Obtainable check.
- [ ] The starter's Template loads in its test.
- [ ] A World with no Game has no Template.
- [ ] Existing trees are unchanged.
