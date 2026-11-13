# 02: Hand crafting end to end

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Phase 16](../spec.md).

## What to build

A player presses I, opens the Crafting tab and crafts a Terra Workbench from 4
Wood. Line 0.3's Declaration gains `recipes` rows (inputs, one output, a
required `station` that is `"hand"` or a tag), also as `[[recipes]]` in
data-only content, resolved after every Mod and validated with the spec's
limits. The Server answers a Craft intent (one, or as many as affordable up to
one output Stack): it spends and gives in one step, drops what does not fit at
the Player, and refuses "Missing items." A Station Recipe is always refused
here; ticket 04 opens it. The Inventory window's side pane gets the Crafting tab
as the mockup shows: beside the Catalogue in Creative, alone in Survival, with
search, affordability, have-against-need, Craft 1 and Craft N, double-click and
Shift-click. Terra declares the Workbench Item and Material (placeable, not yet
usable) and its hand Recipe.

## Acceptance criteria

- [ ] Recipes load from WIT and data-only content; an unknown Item or tag, an
      empty or oversized input list, an output over `max-stack` and a Mod over
      its cap are load errors naming the Recipe.
- [ ] Crafting spends exactly the inputs and gives the output in both Game
      modes; Craft N is bound by one output Stack.
- [ ] Output that does not fit drops at the Player and the Client says so.
- [ ] A craft never enters the undo journal.
- [ ] The Crafting tab follows the mockup and draws only what the Server sends.
- [ ] Terra: crafting a Workbench from Wood works in a new World.
