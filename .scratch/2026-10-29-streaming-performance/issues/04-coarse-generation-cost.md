# 04: Profile and cut coarse generation at identical output

Status: ready-for-agent
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-coarse-generation`.

## What to build

- First, a breakdown of a coarse job per level: 2D height, 3D caves,
  downsample, the uniform early-outs. Also the share of coarse generation spent
  inside the region the finer level draws: `lod.rs`'s `dispatch` generates the
  whole load extent with no hole, which may be prefetch for when the Eye moves.
  Append both here.
- Build a cut only for a stage over 20% of coarse worker time, at identical
  output. Candidates:
  - level 1 only: generate its eight children as one 64-cubed pass in
    `tesserae-core` worldgen, with one smoothed height field instead of eight
    34-by-34 ones (about 13% fewer 2D samples) and one cave bound. Levels 2 and
    up run with `smoothing: None`, so they have no border overlap;
  - generating less inside the finer level's region, if the breakdown shows it
    is not needed as prefetch: a hole-size decision for you.
- A shared pass applies to untouched Chunks only. It excludes saved-change
  branches (ADR-0011's Persistent Worlds section) and level-1 jobs reading held,
  edited level-0 children (`generate_coarse`'s `held`).

## Acceptance criteria

- [ ] Output is bit-equal to today's across several seeds and levels 1 to 3.
- [ ] `tesserae-core` gains no Bevy, wgpu or rayon dependency.
- [ ] Coarse worker CPU per Chunk passes the spec's effect gate, using the core
      comparison methodology.
- [ ] The breakdown is recorded even if nothing is built.

## Validation and authorization

Local work only; no push or PR without approval.
