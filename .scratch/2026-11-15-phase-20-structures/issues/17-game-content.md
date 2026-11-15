# 17: Every Game ships Templates and a Settlement

Status: ready-for-agent

Blocked by: 04, 05, 06, 07, 12, 13, 15, 16

Spec: [Phase 20](../spec.md).

## What to build

Each of Terra, Tidewater, Ember, Lumen and Meadow declares fifteen Templates
from Materials Obtainable in it, as the approved mockup shows: ten Settlement
buildings and five wilderness Structures. Each building kind has its own shape:
a steep-roofed cottage, a jettied half-timbered family house, an L-shaped
farmhouse with a porch and cellar, an inn with a hipped roof and cellar, a long
timber-framed warehouse of chests, a lean-to blacksmith with a forge chimney, a
brick bakery with its oven at the back, a market stall with an awning, a barn
with a two-pitch roof, and a well. Each kind uses its own Materials from ticket
16, so no two buildings in a Game share walls and roof. Terra's stone bridge
humps over a brick-ringed arch, Meadow's footbridge stands on log trestles, and
Tidewater's rope bridge sags between two raised landings. Each declares at least
one Settlement that uses them, and Terra also a town. Stations a building needs
(an anvil, a forge, an oven) are declared with the step 16 machinery. Each
Game's Biomes list them. The SDK starter declares one Template as the documented
example. The Engine ships none.

## Acceptance criteria

- [ ] A World of each Game shows its Templates and a Settlement, level, whole
      and furnished, with cellars underground.
- [ ] Each Template matches its approved mockup in shape and Materials, by a
      test that compares the declared cells to the mockup's export or by review
      recorded under Comments.
- [ ] Every Game passes the Obtainable check.
- [ ] The starter's Template loads in its test.
- [ ] A World with no Game has no Template.
- [ ] Existing trees are unchanged.
