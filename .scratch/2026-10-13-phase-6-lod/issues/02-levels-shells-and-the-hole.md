# Levels, shells and the hole

Status: done
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

- Built as `LevelRings::new(settings, cell_size_m, level)` in
  `server/lod_rings.rs`, not `Rings::for_level(settings, voxel_size, level)`:
  `Rings`' fields are `Extent`s that the existing streaming code and tests use
  as such, and a shell is not one. `Rings::new` now delegates to level 0 of
  `LevelRings`, so there is one derivation. It takes the level-0 Cell in
  metres, as `Rings::new` did, so callers pass `world.cell_size()`. The anchor
  is `WorldSettings::chunk_at_level(metres, level)`, with `cell_size_at(level)`;
  `chunk_at` is level 0 of it.
- `Shell` is `outer` plus `hole: Option<Extent>`. `level_count` is a loop, not a
  `log2`, capped at 32 so a zero view distance cannot spin.
- The hole is `floor(finer_outer / 2) - 1` per axis, i.e.
  `(n - 2).div_euclid(2)`, and `None` if either comes out negative. The spec's
  vertical 1.5 - 1 = 0.5 is floored to 0. `Extent::contains` squares its
  radius, so a negative one must never be stored: the keep hole is `None` when
  shrinking by the hysteresis would go below 0.
- "Overlaps by at least one of this level's Cells" holds as the spec counts it,
  in Cells of radius (hole + 1 Cell <= finer outer, in metres). It is not a
  worst-case guarantee for an Eye anywhere in its Cell: the two anchors differ
  by up to a finer Cell. Ticket 08 owns what the seam needs.
- Two `StreamingSettings` literals in `streaming.rs` tests gained
  `..default()` for the new field; no assertion changed.
- Tests pin, at the defaults (1.0 m Voxels): 6 levels; mesh outer horizontal
  6, 6, 6, 6, 6, 4 and vertical 3; keep outer +1, load +2, evict +3 on each
  axis; mesh hole (2, 0) and keep hole (1, 0) at levels 1 to 5; level 0 has
  none. At 0.25 m Voxels: outer 24 / 12 for levels 0 to 4 and 16 / 12 for
  level 5, level 3's hole (11, 5). Level 0 equals the old derivation for four
  settings at three Cell sizes. `lod_distance_m` <= 192 is one level, 192.5 is
  two.
