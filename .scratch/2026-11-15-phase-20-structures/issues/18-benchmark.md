# 18: Benchmark pass

Status: done

Blocked by: 07, 15

Spec: [Phase 20](../spec.md).

## What to build

A bench of the 9 surface Chunks, bare against with Structures and against a
Settlement, at 1.0, 0.5 and 0.25 m, interleaved CPU time, with a dense and a
sparse spacing and a Template at the cap. The margin is recorded in DESIGN.md.

## Acceptance criteria

- [x] The bench runs with `cargo bench -p tesserae-core` and prints the median
      per variant.
- [x] A Chunk with Structures, and one in a Settlement, costs no more than the
      agreed margin over bare, recorded in DESIGN.md.
- [x] Air and stone Chunks keep their early-outs.

## Comments

Bench: `crates/tesserae-core/benches/structures.rs` (`cargo bench -p
tesserae-core --bench structures`). Templates and Settlements are built from
core types (no Game): a 7 m house at 64 m (sparse) and 14 m (the densest
spacing it may have), a 64 x 16 x 64 m hall at both caps (65,536 cells, every
one written) at 128 m, a 32 m village of 12 Plots and a 64 m town of 24
Plots. Each variant's 9 Chunks centre on what it stands nearest the origin,
found at 1 m, so every Voxel size times the same ground. Each runs warm (the
same generator every round) and cold (a new generator, so the first Chunk
lays the cells out), each beside a bare run of the same Chunks; the ratio is
the median of per-round ratios. `TESSERAE_BENCH_BARE=1` runs only the bare
World, for an A/B against another build.

Tests: `crates/tesserae-core/tests/settlement_worldgen.rs`
(`chunks_are_the_same_from_threads_sharing_one_generator_in_any_order`,
`sky_and_deep_rock_beyond_a_settlement_keep_their_early_outs`).

Decisions:

- Layout cache (`worldgen/layouts.rs`): without it, every Chunk a Structure
  or Settlement reaches accepted or laid it out again; the hall cost 0.63,
  0.59 and 0.57 ms a Chunk at 1.0, 0.5 and 0.25 m and the town 0.50, 0.37
  and 0.36. Each generator holds an `Arc<Layouts>` shared by its clones
  (streaming clones the generator per Chunk job) and replaced by
  `with_templates` / `with_settlements`. It keeps, by row index and lattice
  cell, each Structure (16,384), Settlement site (4,096) and laid-out Town
  (32, as an `Arc`), and clears a map whole when full. Values are pure
  functions of the generator and key, worked out outside the lock, so order
  and threads change nothing. `Claims::towns` is now `Vec<Arc<Town>>`.
- Margin (proposed and recorded in DESIGN.md): Structures or a Settlement
  add at most 0.25 ms to a surface Chunk at any Voxel size, warm or cold;
  below the 0.30 ms the 16-feature Biome's Plants add at 1 m.

Measured (load 1.6 to 1.7; ms a Chunk, warm / cold, bare in brackets):

| Variant           | 1.0 m              | 0.5 m              | 0.25 m             |
| ----------------- | ------------------ | ------------------ | ------------------ |
| house, 64 m       | 0.21 / 0.24 (0.21) | 0.13 / 0.15 (0.11) | 0.10 / 0.12 (0.06) |
| house, 14 m       | 0.30 / 0.37 (0.23) | 0.15 / 0.19 (0.08) | 0.14 / 0.16 (0.07) |
| hall at the cap   | 0.26 / 0.36 (0.25) | 0.21 / 0.28 (0.13) | 0.20 / 0.26 (0.08) |
| village, 12 Plots | 0.23 / 0.28 (0.21) | 0.17 / 0.22 (0.11) | 0.15 / 0.20 (0.08) |
| town at the cap   | 0.24 / 0.31 (0.23) | 0.11 / 0.18 (0.09) | 0.10 / 0.17 (0.07) |

Worst: the hall cold at 0.25 m, 0.18 ms over bare. Sky and deep-rock
Chunks: 0.01 us with or without Structures (deep rock at 0.5 and 0.25 m
carves caves, equal with and without).

Open, for whoever picks it up: bare generation is 2.0, 5.1 and 8.5% above
main at 1.0, 0.5 and 0.25 m (about 5 us a Chunk; interleaved process A/B of
separate builds, 10 to 16 runs each, spreads not overlapping). This ticket
does not change it (the integration base measures the same). It bisects to
15c57f7b (Settlement levelling), which changes nothing on the bare path; the
machine code of `Worldgen::chunk` shows the column loop addressing the Chunk
builder off the stack instead of a register. Tried and dropped: claims in an
`#[inline(never)]` helper, the column loop in its own function (worse at
0.5 m), `#[inline(always)]` on `claims_near`/`stand_claims`,
`-align-loops=64`.

