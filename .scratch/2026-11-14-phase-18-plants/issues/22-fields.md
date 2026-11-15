# 22: Fields: wild species fields and planted crop plots

Status: done

Blocked by: None (20 and 21 are built and the maintainer approved the contract)

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

- [x] A headless test: a 48 m square of a field Biome is mostly one species per
      region with a few strays, at three Voxel sizes.
- [x] Plots are rectangles of one crop in rows, a path and a ditch separate
      them, the same across Chunk borders.
- [x] A Biome without the fields generates its recorded digests.
- [x] The snapshot carries both; older saves load as before.
- [x] Meadow, tidewater and Terra's meadow place wheat fields most often.

## Comments

Built, as the contract above. `surface-feature.field: option<field-def>` and
`biome-def.crop-fields: list<crop-field>` (WIT in both copies, the TOML rows,
`resolve_biomes`, core `Field`, `CropField` and `FieldCrop`). `ModError::BadField`
and `BadCropField` name the Mod, Biome and row. A field group is a name in a
Mod and a small number in the Biome, in order of first appearance.

Wild fields. The filter is in `SurfaceScatter::pick`, so the Chunk pass, the
plant pass and spawn clearance agree. A drawn member the region does not hold
leaves the column bare. Every member of a group agrees on `scale-m` and
`strays`, which is a load error otherwise. Members stand in a share of the
ground, so the demo Games' `grouped` helper closes each spacing by the root of
`(1 + strays * (n - 1)) / n`.

Planted fields (`worldgen/fields.rs`). A lattice cell of `spacing-m` holds a field
half the time and tries 36 places for it; a place is accepted when columns a
metre apart are in the Biome, above the lake, uncarved and level within a metre,
near the lake if asked, and no tree trunk stands on any column. Lanes one Voxel
wide ring the field and divide its plots; the lowest edge is the ditch. A later
entry of a Biome gives way to an earlier one it would cross. Wild plants do not
stand on a field, and the crops go in after the Biome's plants.

Differences from the proposal, each recorded in the ADR-0027 amendment: level is
within a metre (a Voxel at 1.0 m) rather than one Voxel at every size; a tree
skips the place rather than the field, so the cell looks elsewhere; and a held
cell tries up to 36 places, because Terra's temperate ground is level over 12 m
about one place in nine and tidewater's lagoon one in sixty-five. Tidewater's
plots are 6 to 8 m for that reason. No `FORMAT_VERSION` or `GENERATOR_VERSION`
change: both are optional snapshot members written only when present, and a
Biome with neither generates its recorded digests.

Content. Terra's temperate Biome (a field by the lake with a ditch of its water,
and one anywhere), meadow's meadow and prairie, tidewater's lagoon (ditch of the
sea) and lumen's garden lay wheat, carrot and potato plots at 6, 2 and 2 (prairie
8, 1 and 1) on a sand path. Ember has none: its ash is not farm land. Flowers and
vegetables are field groups in Terra's temperate and savanna Biomes, meadow's
meadow and prairie and lumen's garden. In a 1 km square at 1.0 m the fields change, in Voxels of wheat, carrot and potato:
Terra 534, 132 and 288, meadow 2226, 881 and 751, tidewater 624, 259 and 185,
lumen 5326, 1293 and 1493.

Tests: core `tests/fields.rs` (a 48 m square mostly one species per region with
strays and a control, at 1.0, 0.5 and 0.25 m; a lone member unchanged; group and
range errors; plots one crop in rows, a closed ring, one ditch edge, lanes
exactly one Voxel, plot sides in range, a field across a Chunk border, at three
sizes; the same cells whichever Chunks are made first; no field under the lake
and near-water means near; a trunk never in a field; the snapshot freezes both
and an older one has none; an empty list generates what none did; field range
errors; the Biome's size), mods (resolve, freeze, errors, TOML) and
`demo_games::fields` (wheat most often with a path, and a ditch beside a lake;
the groups each Biome declares). Persistence round trips a field and a group.
Cost, from the measured Chunk of each Game over 160 m round the origin on a loaded
host: Terra 0.85 ms against 0.73, meadow 0.92 against 0.77, tidewater 0.93 against
0.89, lumen at 0.25 m 1.16 against 1.03.

Re-pinned: nothing; no recorded digest or table moved.
