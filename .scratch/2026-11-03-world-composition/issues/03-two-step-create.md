# 03: Create World in two steps with a conflict check

Status: done

Blocked by: 01

## What to build

Step 1 picks a Game (name and seed stay there). Step 2 lists required Mods locked and optional Mods tickable. Before Create, run the loader's rule checks over the chosen data Mods and grey out a clashing Mod, naming the one it clashes with. Both pages scroll.

## Acceptance criteria

- [x] Next and Back keep the form's name, seed and picks.
- [x] A Mod whose rules clash with a ticked or required Mod cannot be ticked, and says why.
- [x] The Create button is always visible.
