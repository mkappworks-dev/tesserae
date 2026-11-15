# 22: Fields: wild species fields and planted crop plots

Status: ready-for-agent

Blocked by: 20, 21 (content), and a maintainer go-ahead; specified only

Spec: [Phase 18](../spec.md), amendment 2026-11-15.

## What to build

Two worldgen features, both pure functions of the seed and a column in metres,
so whole across Chunk borders and equal at every Voxel size.

### (a) Wild fields

Ground cover is chosen per region so a patch is mostly one species, as the
mockup's 48 m flower map shows.

Proposed contract: an optional `field` on `surface-feature`:

```
record field-def {
    group: string,      // features of a Biome with the same group compete
    scale-m: f32,       // region size, 8 to 96
    strays: f32,        // 0 to 0.5: the share that ignore the region
}
```

Within a group, each region (the nearest of a jittered lattice of `scale-m`) is
assigned one member by hash; a column that draws a feature of the group stands
it only if it is the region's member, or with chance `strays` any member. A
feature with no `field` is unchanged. The Biome snapshot freezes `field` as an
optional member of the feature.

### (b) Planted fields

A surface feature that generates rectangular plots of one crop in rows, with a
path between plots and a water ditch along one edge, near water in meadow-type
Biomes, wheat most often.

Proposed contract: an optional list on `biome`, `crop-fields: list<crop-field>`:

```
record crop-field {
    crops: list<tuple<string, f32>>,  // crop Plants and weights
    plot-m: tuple<f32, f32>,          // plot side range, 6 to 24
    spacing-m: f32,                   // mean distance between fields, 32 to 256
    path: option<string>,             // a solid Material the path is laid in
    ditch: option<string>,            // a fluid Material filling the edge ditch
    near-water-m: option<f32>,        // only within this of the lake
}
```

A lattice cell of `spacing-m` draws whether it holds a field; a field's plots
are a small grid of `plot-m` squares separated by a one-Voxel-wide `path`, each
plot one crop (weighted) planted every second row at its ripe or a hash-drawn
Stage, a ditch along its low edge cut one Voxel below the ground and filled
with the fluid. Plots lie only on columns whose ground is level within one
Voxel and whose Biome allows the feature; plots over water, trees or carved
cells are skipped whole. The generator version question must be answered first:
it adds Voxels to existing Biomes only when a Biome opts in, so a World made
before has none.

## Acceptance criteria

- [ ] A headless test: a 48 m square of a field Biome is mostly one species per
      region with a few strays, at three Voxel sizes.
- [ ] Plots are rectangles of one crop in rows, a path and a ditch separate
      them, the same across Chunk borders.
- [ ] A Biome without the fields generates its recorded digests.
- [ ] The snapshot carries both; older saves load as before.
- [ ] Meadow, tidewater and Terra's meadow place wheat fields most often.

## Comments

Not built. Contract above is a proposal for the maintainer to confirm before any
code.
