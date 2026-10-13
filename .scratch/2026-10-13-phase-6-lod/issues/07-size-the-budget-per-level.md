# Size the budget per level

Status: done
Blocked by: 04, 06, 08

Spec "Performance". The per-ring budget has only ever been measured for level 0.

## What to build

- Per level 1 to 5: Chunks held, non-uniform, meshed non-empty; generation and
  mesh microseconds per Chunk; quads and vertex+index bytes per mesh; time to
  fill that level alone. Total vertex memory at the default reach. Extend the
  `dev` capture report with per-level counts; measure core costs with
  `benches/lod.rs` (`lod/coarse_chunk`, `lod/mesh_coarse`).
- Sweep `LodBudget` in-flight and landing caps with `scripts/frame-runs`, five
  runs per configuration, interleaved; `TESSERAE_LOD=0` vs `1` for the effect on
  level 0's fill and worst tick (`MainThreadCost`, not frame time).
- Decide whether level-0 latency needs a second pool for coarse work; if the A/B
  shows no effect, say so and leave one.
- Set the defaults. Write the doc comment on `LodBudget` in the form of
  `StreamingBudget`'s, with the staleness window computed from the measured
  costs and no number whose reason is gone.
- Update DESIGN.md's per-ring open item with the table.

## Acceptance criteria

- Sweep tables in Comments with machine and load average.
- `./scripts/cargo-ci` passes.

## Comments

The tables, the load averages and the pool decision are in the spec's Outcome,
and the budget table is in `LodBudget`'s doc comment and DESIGN.md's open item.
The sweeps were run twice, the second time as five interleaved runs of seven
rows on the same build at load 3 to 7. Level 0's shown time agreed within 0.1 s
everywhere but the last two rows (0.13 s and 0.39 s), and the fills within 0.2 s
except the last row (3.4 s against 3.1 s). The 8, 16 row did not finish filling
inside the capture window the second time, so its 20.8 s stands on the first set
alone.

| In flight / landed (gen, mesh)   | level 0 shown | levels filled | worst tick |
| -------------------------------- | ------------- | ------------- | ---------- |
| no coarse levels                 | 1.33 s        | -             | 0.8 ms     |
| 8, 16 / 8, 8                     | 1.46 s        | 20.8 s        | 1.05 ms    |
| 16, 32 / 16, 16 (the defaults)   | 1.66 s        | 11.1 s        | 0.9 ms     |
| 24, 48 / 24, 24                  | 1.75 s        | 8.0 s         | 0.8 ms     |
| 32, 64 / 32, 32                  | 1.96 s        | 6.4 s         | 1.0 ms     |
| 64, 128 / 64, 32                 | 2.45 s        | 4.6 s         | 1.2 ms     |
| 256, 512 / 512, 128              | 3.61 s        | 3.1 s         | 1.2 ms     |
