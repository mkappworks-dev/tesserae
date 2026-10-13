# Levels, shells and the hole

Status: ready-for-agent
Blocked by: none

Spec decision 4 and 11. Pure arithmetic in `crates/tesserae`; no streaming yet.

## What to build

- `StreamingSettings.lod_distance_m` (default 4096). Count of levels is
  `1 + ceil(log2(reach / view_distance_m))`, 1 when `reach <= view_distance_m`.
- `Rings::new` becomes `Rings::for_level(settings, voxel_size, level)` (keep a
  `new` for level 0 if it keeps call sites small). A level's Cell is
  `cell_size * 2^level`; distances scale by `2^level`; the outer extent is
  `ceil(min(view_distance_m * 2^k, reach) / cell_k)` Cells horizontally and the
  same Cell count as level 0 vertically; hysteresis is the same Cell count.
- The mesh shell is `outer` minus a hole (an `Option<Extent>`): the previous
  level's outer extent in this level's Cells, less one Cell on each axis. The
  keep shell shrinks the hole horizontally only, by the hysteresis. `load` and
  `evict` have no hole.
- A way to ask a shell whether an offset is in it, and a level's anchor Chunk
  (`floor(eye / cell_k)`) in the same style as `WorldSettings::chunk_at`.
- Everything is derived from metres; no Voxel count appears.

## Tests

- Level 0 through the new constructor equals today's `Rings` for several
  settings (this is how "level 0 is unchanged" is checked).
- At the defaults: 6 levels; outer horizontal 6 for levels 1 to 4 and 4 for
  level 5; vertical 3; hole (2, 0); hysteresis 1 Cell at every level.
- Voxel size 0.25 gives the same distances in metres and a quarter of the Chunks
  per metre; a table over `{1.0, 0.25}` and levels.
- `lod_distance_m <= view_distance_m` gives one level.
- The hole is inside the previous level's outer extent in metres (never past it)
  and overlaps it by at least one of this level's Cells.
- Anchors floor, not truncate, below the origin.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- No streaming behaviour changes.

## Comments
