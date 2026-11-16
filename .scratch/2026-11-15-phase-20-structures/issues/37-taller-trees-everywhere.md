# 37: Every tree stands tall enough to walk under

Status: done

Blocked by: 28

Spec: [Phase 20](../spec.md).

## What to build

Play-testing after ticket 28: "even swamp has this problem ... can you do the
same for other biomes. make the tree taller". Every `flora` tree keeps its
leaves, limbs and vines at least 4 m up, at 1.0, 0.5 and 0.25 m, within
`MAX_PLANT_HEIGHT_M`, in every Biome of every Game.

## Acceptance criteria

- [x] Under every `flora` tree, nothing but its trunk is lower than 4 m, at
      every Voxel size, tested.
- [x] Each Biome keeps its tree density class.
- [x] No recorded digest moves.

## Comments

`mods/flora/content.toml`, trunk heights before → after, and limbs:

| Tree   | Trunk            | Branching                    |
| ------ | ---------------- | ---------------------------- |
| oak    | 4–6 → 10–13 m    | start 0.25 → 0.6, fork 0.6   |
| birch  | 5–7.5 → 9–12 m   | start 0.5 → 0.6              |
| pine   | 7–10 → 12–16 m   | start 0.25 → 0.5             |
| palm   | 6–9 → 9–12 m     |                              |
| acacia | 3.5–5.5 → 7.5–9 m | fork 0.25 → 0.6             |
| swamp  | 3–4.5 → 12–15 m  | start 0.3 → 0.7, vines 2 m   |
| apple  | 4–6 → 8–10 m     | start 0.3 → 0.7              |
| orange | 3–4.5 → 8–10 m   | start 0.3 → 0.7              |
| cherry | 4–6 → 8–10 m     | start 0.3 → 0.7              |

Measured lowest leaf, limb or vine over 200 trees each (1 / 0.5 / 0.25 m):
oak 4 / 5 / 5, birch 4 / 5 / 5.25, pine 4 / 4.5 / 4.75, palm 7 / 7 / 5,
acacia 5 / 4.5 / 4.5, jungle 6 / 6.5 / 6, swamp 6 / 5 / 5, apple, orange and
cherry 4 / 5 / 5. Fruit now hangs 4 m and more up, within the 8 m Edit reach.

Test: `under_every_tree_nothing_but_its_trunk_is_lower_than_four_metres`
(`demo_games/vegetation.rs`) counts as trunk only wood within the tree's
`lean-m` of its base column, so a leaning palm or birch trunk is allowed and
a limb is not. Tidewater's `cold_shallows` pines close from 8 to 7.5 m
apart, as taller trees left it at 11.4 m, past its moderate class.

As with ticket 28, a World made before stops opening: `flora`'s build hash
changed.
