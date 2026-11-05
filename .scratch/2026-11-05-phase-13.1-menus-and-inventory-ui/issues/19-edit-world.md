# 19: Edit World

Status: ready-for-agent

Blocked by: 16, 06

Spec: [Phase 13.1](../spec.md).

## What to build

Edit World offers the name, the World's default game mode and the player's own mode, with the Game, seed, Voxel size and Biomes shown as locked. Mode changes are written to the sidecar as pending and applied by the Server when the World next opens; no Client changes a mode during play. Create from these settings starts Create World prefilled. Delete asks first.

## Acceptance criteria

- [ ] Renaming keeps the terrain.
- [ ] A changed default or own mode is in effect after the next open, and not before.
- [ ] Create from these settings prefills every step.
- [ ] Delete needs confirmation and refuses an open World, as today.
