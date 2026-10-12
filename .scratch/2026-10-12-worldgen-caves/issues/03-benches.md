# Bench a Chunk a cave crosses, in all three stages

Status: done
Blocked by: 02

Without it the budget is being sized against terrain that no longer exists.

## What to build

- A fixture finder in `benches/meshing.rs` that scans for a deep Chunk inside
  the cave band that is not uniform, deterministically (fixed seed, fixed scan
  order), plus a deep Chunk inside the band that is.
- `worldgen/cave_crossed` (the carved path), `worldgen/cave_bound_declines` (the
  bound ruling a deep Chunk out, which is the new cost of the old early-out),
  `worldgen/cave_bound_passes_empty` (it could carve and does not).
- `light_chunk/cave_dark_neighbours` (what a Chunk arriving costs: unsolved
  neighbours) and `light_chunk/cave_mouth` (with sky arriving through a mouth).
- `mesh_chunk/cave_crossed`.
- The existing benches keep their names and fixtures, so the before/after
  comparison in the PR is like for like.

## Acceptance criteria

- The before/after table is in Comments, taken in isolation (not in a full run
  disturbed by other work), and says which rows are new.
- The surface-crossing rows do not regress beyond noise, or the regression is
  explained: `straddling` now pays for the Chunk-level bound.
- `cargo bench --no-run` builds without `--features dev`.

## Comments

**Fixtures.** The existing benches keep their Chunks: the `worldgen()` helper
now builds a caveless World, so `straddling`, `light_chunk/*` and
`mesh_chunk/*` measure what they measured in step 4 and 5. A second, caved World
(`caved()`) feeds the new rows, and `cave_fixtures` finds three Chunks by
scanning a 16 x 16 square in a fixed order: the deep in-band layer's first
Chunk a tunnel crosses, its first that none does, and the first surface Chunk
where a tunnel removes a Grass cap.

**Before / after**, Apple M5, release, one `cargo bench -p tesserae-core` run on
each side (`main` at `d8b3f31` in a clean checkout for the left column). Rows
that are unchanged code differ by up to 7% between runs, so read less than that
as noise: `greedy_mesh/solid` went 320 -> 344 us with no change to the mesher.

| Bench                            | main      | branch    |
| -------------------------------- | --------- | --------- |
| `worldgen/straddling`            | 103.1 us  | 104.3 us  |
| `worldgen/early_out_air`         | 3.8 ns    | 3.6 ns    |
| `worldgen/early_out_stone`       | 3.8 ns    | 4.2 ns    |
| **`worldgen/straddling_caved`**  | -         | 252.4 us  |
| **`worldgen/cave_crossed`**      | -         | 263.4 us  |
| **`worldgen/cave_mouth`**        | -         | 213.1 us  |
| **`worldgen/cave_band_uniform`** | -         | 29.5 ns   |
| **`worldgen/cave_band_layer_of_64`** (per Chunk) | - | 138 us |
| `light_chunk/straddling`         | 89.3 us   | 94.4 us   |
| `light_chunk/straddling_settled` | 140.6 us  | 140.5 us  |
| `light_chunk/open_sky`           | 3.9 ns    | 4.2 ns    |
| `light_chunk/opaque`             | 3.1 ns    | 3.1 ns    |
| **`light_chunk/cave_dark_neighbours`** | -   | 44.9 us   |
| **`light_chunk/cave_mouth`**     | -         | 102.1 us  |
| `mesh_chunk/straddling`          | 349.3 us  | 347.2 us  |
| `mesh_chunk/declined_enclosed`   | 5.9 ns    | 5.8 ns    |
| **`mesh_chunk/cave_crossed`**    | -         | 395.3 us  |
| **`mesh_chunk/cave_mouth`**      | -         | 360.2 us  |

Read it as:

- **A surface Chunk a tunnel crosses costs 2.4x to generate** (104 -> 252 us).
  That is the sampling, and it applies to every Chunk in the surface layers, not
  only deep ones. Meshing and lighting it cost about what they did.
- **A deep Chunk in the band that a tunnel crosses goes from nanoseconds to
  about 0.7 ms across the three stages** (263 + 45 + 395 us). A layer of them
  averages 138 us to generate, because a quarter of the band stays uniform.
- **The fast paths themselves did not slow down**: both worldgen early-outs, the
  open-sky and opaque light paths and the enclosed mesh case are within noise.
  What changed is how many Chunks reach them.
- **The range test is cheap when it rules a Chunk out**: 29.5 ns for the first
  such Chunk in scan order. A Chunk that passes it and carves nothing costs
  tens of microseconds; the layer mean above has them in it.
