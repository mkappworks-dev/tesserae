# 22: Food in every demo Game

Status: ready-for-agent

Blocked by: 12, 21

Spec: [Phase 13.1](../spec.md).

## What to build

Every demo Game has a food that lies in its World. Terra's melon becomes a surface feature on grass. Each of the other four demo Games declares one food of its own, themed to the Game: a placeable solid Material whose Item has a food value from 4 to 8, with its own texture and appearance colour, scattered as a surface feature in the Biomes whose surface suits it, at a mean spacing comparable to Terra's melons (findable while exploring, not a carpet). Where a Game's ground comes from a shared data Mod, the food lives where it fits best, recorded in this ticket. New Materials append after existing ones so no id moves.

## Acceptance criteria

- [ ] Every demo Game declares at least one food Item with a food value.
- [ ] A new Survival World of every demo Game has its food on the surface within a short walk of spawn.
- [ ] Each food can be placed, broken, picked up and eaten with G.

## Comments

- Maintainer, 2026-11-05: every demo Game gets food, not only Terra. Variety (crops, cooking, meat) waits for steps 16, 18 and 22.
