# 28: Jungle trees stand tall enough to walk under

Status: done

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found the jungle hard to cross: its trees are too short. The
`flora:jungle` tree (shared by Terra, Tidewater, Lumen and Meadow) has a 10 to
16 m trunk whose limbs start at 0.55 of it, lobes of 1.8 m, and solid vines
6 m long, so leaves, limbs and vines reach down to a Player's head.

- Make the jungle tree tall, in metres: a taller trunk, limbs starting
  higher, and vines that stop above a Player's head.
- Worldgen stays deterministic. There is no new Generator version: a World's
  snapshot pins its Plant rows, so a World saved before keeps its old trees and
  its recorded digests.
- If the change would move a recorded digest, stop and ask.

## Acceptance criteria

- [x] Under every jungle tree worldgen stands, nothing of the tree but its
      trunk is lower than 4 m above its base, at 1.0, 0.5 and 0.25 m, tested.
- [x] The tree stays within `MAX_PLANT_HEIGHT_M`.
- [x] Every recorded digest test still passes unchanged.

## Comments

`flora:jungle` in `mods/flora/content.toml`: trunk 10 to 16 m became 18 to
24 m, limbs start at 0.6 of it (was 0.55), and vines hang at most 4 m (was 6
m). Before, leaves and limbs began 4 to 6 m up and vines reached 1 to 2 m off
the ground; now the lowest leaf, limb or vine stands 7 m or more up at every
Voxel size, measured over 400 trees each at 1.0, 0.5 and 0.25 m by
`under_a_jungle_tree_nothing_but_its_trunk_is_lower_than_four_metres`
(`demo_games/vegetation.rs`), which also keeps the tree within
`MAX_PLANT_HEIGHT_M`.

No digest moved: `generator_version`, `surface_features`, `coarse_generation`
and `plant_worldgen` pass unchanged. A World's snapshot pins its Plant rows,
so a World made before keeps its shorter jungle; no new Generator version.

Found after closing: a World pins each Mod's build hash (`Content::require`),
so changing `flora` stops a World made before from opening ("required Mod
flora 0.2.0 (...) is missing or changed"); the play-tested `tt` and `New
World` are such. This branch's Template changes do the same through Terra's
build. Decided 2026-11-16 by the user: accept it; play-test in a new World.
