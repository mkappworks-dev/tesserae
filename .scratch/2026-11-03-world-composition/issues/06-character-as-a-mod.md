# 06: A character can come from a Mod

Status: done

## What to build

Allow one non-root Mod to supply `world.character` and nothing else from `world`. At most one source per World; the Game's own counts as a source. Add character Mods and have the demo Games depend on one. Needs an ADR and a compatibility note, and a decision on how a Mod's `world` is limited to `character` in the 0.3 contract.

## Acceptance criteria

- [x] ADR records the rule and how a Mod's `world` is limited.
- [x] Two sources fail the load naming both.
- [x] Each demo Game other than terra shows a Mod's character.
