# 19: Edit World

Status: done

Blocked by: 16, 06

Spec: [Phase 15](../spec.md).

## What to build

Edit World offers the name, the World's default game mode and the player's own mode, with the Game, seed, Voxel size and Biomes shown as locked. Mode changes are written to the sidecar as pending and applied by the Server when the World next opens; no Client changes a mode during play. Create from these settings starts Create World prefilled. Delete asks first.

## Acceptance criteria

- [x] Renaming keeps the terrain.
- [x] A changed default or own mode is in effect after the next open, and not before.
- [x] Create from these settings prefills every step.
- [x] Delete needs confirmation and refuses an open World, as today.
