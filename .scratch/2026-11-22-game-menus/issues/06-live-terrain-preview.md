# 06: Live terrain preview

Status: ready-for-agent

Blocked by: [05: Create World: Game step and
titles](./05-create-world-game-step.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

A pure function turns (Game, seed, climate scale, enabled biomes) into a small
top-down image from the same Generator the World uses. The World step shows it
beside the seed field and redraws it as the seed changes, off the main thread.
Randomize and Copy are icons inside the seed field, and climate scale moves from
the Biomes step to the World step beside voxel size, both marked fixed at
creation.

## Acceptance criteria

- [ ] The same inputs give the same image
- [ ] The preview agrees with the Generator at sampled points
- [ ] Typing a new seed redraws the preview without blocking input
- [ ] Climate scale is edited and validated on the World step

## Comments
