# 06: Live terrain preview

Status: done

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

- [x] The same inputs give the same image
- [x] The preview agrees with the Generator at sampled points
- [x] Typing a new seed redraws the preview without blocking input
- [x] Climate scale is edited and validated on the World step

## Comments

- The pure function is `tesserae_core::terrain_preview::Source::render(seed, voxel_size, edits, frame)`. `Source` is read with the Biomes, by the same declaration pass (`BiomePreview.source`), and keeps the Game's Templates and Settlements so the picture centres on the spawn the Server finds.
- Each pixel is its column's `Worldgen::column_top`, as the Map and LOD read it: what stands on the ground and cave mouths are not drawn. The agreement test therefore checks against generated Chunks with caves off.
- Leaving the Game step now trusts and reads the Game together with the Mods it requires: Terra cannot be read without `strata`. The Game step's Trust dialog lists those Mods too.
- A Terra render takes 1 to 2 s in the dev profile, on the async compute pool. Only one runs at a time; the newest inputs are drawn when it lands. A coarse first pass would make typing feel quicker if that matters.
- The "More World options" disclosure is gone: Voxel size and Climate scale are inline, as the mockup has them. Resetting the Biomes still restores the Climate sizes, as its dialog says.
