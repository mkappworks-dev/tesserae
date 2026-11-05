# 21: Surface features

Status: ready-for-agent

Blocked by: 08

Spec: [Phase 13.1](../spec.md).

## What to build

A Biome can declare surface features: a Material scattered one Voxel above that Biome's surface, deterministic from the World seed, at a mean spacing in metres. The declaration is appended to the Biome record in the 0.3 contract, with the data-only form, the Rust SDK, the starter, the authoring guide and "Amended before release" in the contract compatibility doc. Core worldgen places a feature only where the column's top Voxel is a full solid Voxel of the Biome's surface Material with air above it, so never under a fluid, on a smoothed slab or slope surface, or in a cave opening. Because features belong to the Biome, the World's pinned Biome table carries them (ADR-0022); a save whose snapshot has none generates exactly the terrain it did before, so the Generator version does not change. Features are generated at the finest level only; coarse distance levels (ADR-0011) leave them out. Mods still cannot place terrain: the engine places what the declaration says.

## Acceptance criteria

- [ ] A Biome declares surface features in the 0.3 contract and the data-only form; out-of-range spacing, an unknown or non-solid Material, and too many features are refused at load.
- [ ] Generation places features one Voxel above the Biome's full solid surface at the declared mean spacing, deterministically from the seed, and never floating, underground, under a fluid or on a smoothed surface.
- [ ] Features are absent at coarse distance levels.
- [ ] A World saved before this ticket generates identical terrain after it.
- [ ] Removing a feature drops its Item, as for any Voxel.

## Comments

- Maintainer, 2026-11-05: chosen over a breaking drop, a starter kit or waiting for step 18, so that food lies in the World. Plants (step 18) adds growth and harvesting on top.
