# Baselines, and the sweep and ray benchmarks they never had

Status: done

Spec decisions 7-9 change `collide.rs`, which has no benchmark, and decision 6
asks that pass 1 not move, which needs a number to not move from. This ticket
writes no feature code.

## What to build

- Measure `main` before the change, fresh (the step-4/5 numbers quoted in older
  briefs are stale). A saved criterion baseline was the plan; **in practice the
  `main` and branch bench binaries are run alternately** (interleaved A/B), which
  needs no baseline in `target/` and survives a noisy machine. Either way the
  table of medians goes in a `## Comments` section of this ticket so the numbers
  survive the machine.
- Add benches to `crates/tesserae-core/benches/meshing.rs` (or a new `collide`
  bench file registered in `Cargo.toml`) **on the unchanged code**, and run them
  against `main` too: a fall onto a floor of cubes, a walk along a wall,
  a ray across terrain, and the same three on a floor of slabs. Same scenes
  after the change; they are the "before" for tickets 03, 06 and 08.
- Confirm the runs agree within noise (medians close to minimums), so a "no
  change" means something. If the machine is noisy, say so here and wait for a
  quiet one.

## Done when

The before/after table is in this file, from a quiet machine, with differences
that cannot be explained stated as such; the new benches compile under `cargo
bench --workspace --no-run --locked`; and `./scripts/cargo-ci` passes. A saved
criterion baseline is not required.

## Comments

Done, with the scope adjusted as above (interleaved A/B instead of a saved
baseline). The collide benches are written
(`crates/tesserae-core/benches/collide.rs`, registered in `Cargo.toml`) and
compile; scenes are a one-Chunk room of cubes or slabs, plus terrain.

**The baseline taken in this session is void and was deleted.** The machine's
load average was 54 (other sessions compiling and running `target/debug/
tesserae`), and the numbers it produced were roughly 8-10x the step-5 figures
(`shape_table_builtin` 42 ms against 4.8 ms, `greedy_mesh/terrain` 2.9 ms against
a few hundred microseconds), with confidence intervals of +-15% on the 500 ns
benches. A "no change" against that baseline would mean nothing.

Also learned: pass `--bench meshing --bench collide` to `cargo bench`, since a
bare `-- --save-baseline` is also handed to the lib's test harness, which
rejects it.

**Measured, interleaved, on a quiet machine (load 3-5, medians within ~2% of
minimums, five rounds).** No criterion baseline was saved: the `main` bench
binary (origin/main 7a9d313, with the new `collide` bench copied in) and the
branch's merged binary were run alternately instead. Apple M5, release.

| Bench                                     |   main |  branch | change |
| ----------------------------------------- | -----: | ------: | -----: |
| `greedy_mesh/terrain`                     | 333 us |  352 us |  +5.7% |
| `greedy_mesh/solid`                       | 317 us |  320 us |  +0.8% |
| `greedy_mesh/checkerboard`                | 2.63 ms | 2.60 ms |  -1.0% |
| `greedy_mesh/terrain_stairs_at_cap`       | 532 us |  559 us |  +4.9% |
| `greedy_mesh/all_stairs`                  | 5.99 ms | 6.16 ms |  +2.9% |
| `mesh_chunk/straddling`                   | 387 us |  404 us |  +4.5% |
| `light_chunk/straddling_settled`          | 299 us |  302 us |  +1.1% |
| `light_chunk/flooded_air`                 | 123 us |  122 us |  -0.8% |
| `shape_table_builtin`                     | 4.87 ms | 4.28 ms | -12.1% |
| `collide/fall_cubes` / `fall_slabs`       | 67 / 71 ns | 62 / 65 ns | -8% / -9% |
| `collide/walk_into_wall_cubes` / `_slabs` | 74 / 74 ns | 91 / 91 ns | +23% / +23% |
| `collide/ray_cubes` / `_slabs`            | 424 / 418 ns | 423 / 430 ns | -0.3% / +2.9% |
| `collide/ray_terrain`                     | 492 ns |  498 ns |  +1.1% |
| `collide/fall_terrain`                    | 442 ns |  162 ns | -63.3% |

Read honestly: pass 1 itself is unchanged code, and `solid`/`checkerboard` are
flat, but `terrain` reads +2% on minimums and +6% on medians, and
`mesh_chunk/straddling` (which contains it) +4%. I have not found what moves it
and a layout effect from `Variant` growing is the likely cause, so it is a
reported cost and not a proven one. Pass 2 costs about 5% more per shaped
Voxel (`is_connected`, `connections` and `coverage_against` each re-read the
variant). The wall-walk sweep is 17 ns slower over about 30 cells; the fall
sweeps are faster and `fall_terrain` is 63% faster because `obstacles` no
longer allocates. The light solvers did not move.

**After a follow-up** (`coverage_against` does one lookup, and a table with no
connected Shape skips the per-Voxel check; same method, load 3-4): `terrain`
+1.0% on minimums (was +2.2%), `terrain_stairs_at_cap` +4.1% (was +5.1%),
`all_stairs` +3.2% (was +5.7%), `mesh_chunk/straddling` +2.8% (was +3.7%),
`light_chunk/straddling_settled` +0.1%. So the lookups were part of the cost and
not most of it. The rest is unexplained; the `Variant` struct being larger is
the suspect, and reordering its fields or moving the cold arrays out of it is
the next thing to try. The wall-walk sweep is still +24% (about 17 ns) and the
fall sweeps are still faster, which `connections_at` cannot explain now that it
is skipped for a table with no connected Shape.

**Residue explained (see ticket 02 of
`.scratch/2026-10-12-merged-rails-across-voxels/`).** The +1% to +5% on pass-1
benches was code layout, not work: instructions retired for terrain, a full
Chunk and an empty one are 0.02% lower on this branch than on `main`, and cycles
are equal once `mesh_shaped` is inlined as it is on `main`. `Variant` growing
was not the cause (terrain never reads a `Variant`; cubes and air return before
the table).
