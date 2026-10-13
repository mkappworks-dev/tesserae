# Size the budget per level

Status: ready-for-agent
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
