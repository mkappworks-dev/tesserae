# 13: Benchmark pass

Status: ready-for-agent

Blocked by: 05, 07

Spec: [Phase 20](../spec.md).

## What to build

A bench of the 9 surface Chunks, bare against with Structures, at 1.0, 0.5 and
0.25 m, interleaved CPU time, with a dense and a sparse spacing and a Template
at the cap. The margin is recorded in DESIGN.md.

## Acceptance criteria

- [ ] The bench runs with `cargo bench -p tesserae-core` and prints the median
      per variant.
- [ ] A Chunk with Structures costs no more than the agreed margin over bare,
      recorded in DESIGN.md.
- [ ] Air and stone Chunks keep their early-outs.
