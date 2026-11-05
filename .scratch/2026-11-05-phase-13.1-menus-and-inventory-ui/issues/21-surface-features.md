# 21: Surface features

Status: done

Blocked by: 08

Spec: [Phase 13.1](../spec.md).

## What to build

A Biome can declare surface features: a Material scattered one Voxel above that Biome's surface, deterministic from the World seed, at a mean spacing in metres. The declaration is appended to the Biome record in the 0.3 contract, with the data-only form, the Rust SDK, the starter, the authoring guide and "Amended before release" in the contract compatibility doc. Core worldgen places a feature only where the column's top Voxel is a full solid Voxel of the Biome's surface Material with air above it, so never under a fluid, on a smoothed slab or slope surface, or in a cave opening. Because features belong to the Biome, the World's pinned Biome table carries them (ADR-0022); a save whose snapshot has none generates exactly the terrain it did before, so the Generator version does not change. Features are generated at the finest level only; coarse distance levels (ADR-0011) leave them out. Mods still cannot place terrain: the engine places what the declaration says.

## Acceptance criteria

- [x] A Biome declares surface features in the 0.3 contract and the data-only form; out-of-range spacing, an unknown or non-solid Material, and too many features are refused at load.
- [x] Generation places features one Voxel above the Biome's full solid surface at the declared mean spacing, deterministically from the seed, and never floating, underground, under a fluid or on a smoothed surface.
- [x] Features are absent at coarse distance levels.
- [x] A World saved before this ticket generates identical terrain after it.
- [x] Removing a feature drops its Item, as for any Voxel.

## Comments

- Maintainer, 2026-11-05: chosen over a breaking drop, a starter kit or waiting for step 18, so that food lies in the World. Plants (step 18) adds growth and harvesting on top.
- The contract appends `surface-features: list<surface-feature>` to
  `biome-def`, where `surface-feature` is `{ material: string, spacing-m: f32 }`;
  the data-only form takes
  `surface-features = [{ material = "...", spacing-m = 16.0 }]`. No list of
  surface Materials to stand on: a feature stands on its Biome's own surface
  Material (the Biome's `ground`, or the World's for a Biome without one), which
  covers every demo Game.
- Load refuses, naming the Mod and `biome`: more than 4 features in a Biome, a
  spacing outside 4 to 1024 m (4 m at 1 m Voxels is one column in 16, already a
  carpet; 1024 m is one in a square kilometre), and a Material that is unknown,
  Air, not solid or a fluid. A saved snapshot is checked the same way when it
  reopens.
- A column's draw is a uniform number from a hash of the folded World seed and
  the column's metre position. Each feature's chance is
  `(voxel size / spacing)²`, the features' chances laid end to end so each keeps
  its own density, so the mean spacing is the declared metres at any Voxel size.
  The Biome is looked up only for a draw below the largest total. A feature
  stands in the cell over the cube pass's top Voxel when that Voxel is a full
  cube of the Biome's surface Material and the cell is still air after the lake
  and smoothing: never under a fluid, on a stair or slab, or where a cave carved
  the top. The cell's floor must also be inside the surface band, so the
  open-sky early-outs (`surface_bounds_m`) stay true; only a column within one
  Voxel of the very top of the relief is skipped. Like a stair, a cell over the
  last Voxel of the Chunk below is made by the Chunk above.
- The LOD source generators run on `WorldgenParams::without_surface_features`:
  the level 0 that level 1 downsamples, every coarser source and the column
  generators. Downsampling a level 0 with features let two of them in one octant
  win a coarse Voxel's vote. A Chunk a player changed keeps the features it
  held, and downsamples like any other edit.
- No Generator version change: without features no pass runs, and a saved
  Biome omits an empty `surface_features`. `tesserae-core/tests/surface_features.rs`
  pins digests recorded before this ticket for a Biome World with caves,
  smoothing, a lake, Biome grounds and ore, generated directly and through a
  saved snapshot.
- Create World Biome edits keep a Biome's features; the step has no control for
  them.
