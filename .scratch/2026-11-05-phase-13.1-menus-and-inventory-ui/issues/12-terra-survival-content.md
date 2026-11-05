# 12: Terra Survival content

Status: done

Blocked by: 08

Spec: [Phase 13.1](../spec.md).

## What to build

Terra declares at least one food Item with its food value, so hunger can be survived in the reference Game, and declares contact damage for lava in every Game that declares lava. Any other Game's Survival numbers stay at the engine defaults unless this ticket finds a reason to change them, recorded in the ticket.

## Acceptance criteria

- [x] Terra has a food Item that can be gathered or taken from the Catalogue.
- [x] Lava hurts on contact in every Game that declares it.
- [x] Existing World saves of every Game still open.

## Comments

- Terra declares a solid `melon` Material and a `melon` Item that places it, with a food value of 6 (of the default 20 hunger). The Catalogue gives it in Creative, and removing a placed melon drops it for Pickup. Terra generates none: ores sow only rock, and new generation would change the terrain of every Terra World, so a fresh Survival Terra World has no food until a Creative player places some.
- Lava hurts 4 health per second on contact: `terra:lava`, and `hearth:lava`, which is where Ember's lava comes from. No other Game has a lava Material.
- No Game's Survival numbers change: every Game keeps `survival: None` and the engine defaults.
- Every change is additive (a new Material, Item and Material field; no Material renamed, removed or reordered), so the Terra persistence round trips still open. A World saved with an earlier build of Terra or hearth is still refused as a changed build, as for every rebuild (ADR-0018); that is not this ticket's to change.
