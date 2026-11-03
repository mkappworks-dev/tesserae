# 03: Add the ember Game

Status: done

Blocked by: 01

## What to build

Add `games/ember`: a volcanic highland on strata, waters, flora and hearth. Lava vents erupt on a timer from `tick`, and water meeting lava makes obsidian through the shared reactions.

## Acceptance criteria

- [x] Vents erupt on the declared timer, and a test observes an eruption.
- [x] Wood burns and leaves ash through hearth's rules.
- [x] README covers behaviour, dependencies and how to run it.

## Comments

The wood-to-ash rule is checked in the loaded fire table; no burn is simulated in a test.
