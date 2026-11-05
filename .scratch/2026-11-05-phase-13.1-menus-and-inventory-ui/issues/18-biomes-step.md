# 18: Biomes step

Status: done

Blocked by: 17

Spec: [Phase 13.1](../spec.md).

## What to build

Create World gains a Biomes step. The launcher previews the declarations of the selected Game and Mods, under the same trust rules as opening, and lists their Biomes by title-cased declared name. The player can turn each off, move its Climate point, scale or switch off its caves, choose its ground Materials from the selected Game and Mods, change its sky and fog colours, set the Climate sizes in metres, and Reset to defaults. The edits are stored with the pending World; on first open the Server applies them to the Biome table before the snapshot, refusing out-of-range values, unknown Materials and an empty Biome set. Review lists the Biomes. The Game's declarations are untouched (ADR-0022).

## Acceptance criteria

- [x] A World created with a Biome off never generates that Biome; the Game's table is unchanged for the next World.
- [x] Edited Climate points, cave scale, ground and colours appear in the snapshotted table.
- [x] Invalid edits are refused at Create and at open.
- [x] A World created without visiting the step matches today's output for the same seed.
