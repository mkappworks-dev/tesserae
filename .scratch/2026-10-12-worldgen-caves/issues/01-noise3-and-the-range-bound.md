# A 3D value-noise primitive and an exact range over a box

Status: ready-for-agent

Spec decision 8 and the prototype findings. Everything else stands on this.

## What to build

- `noise::noise3`, `noise::fbm3`: the 3D counterparts of `noise2` and `fbm2`,
  same smoothstep, same octave scheme, same `[0, 1)` bound. Core keeps zero
  dependencies.
- `noise::fbm3_range(lo, hi, seed, octaves) -> (min, max)`: an interval
  containing `fbm3` everywhere in the box. Exact per octave (multilinear in the
  smoothstepped weights, so extremes sit at the box's corners within each
  lattice cell; split the box at cell planes), summed over octaves, with a small
  slack for rounding. Sound, not tight.
- `fbm3`'s octave seeds use `wrapping_add`, so a seed near `u32::MAX` cannot
  overflow in debug.

## Acceptance criteria

- `noise3` and `fbm3` stay in `[0, 1)`; lattice points are reproduced exactly;
  a different seed is a different field.
- Soundness: for a few thousand random boxes (including boxes straddling
  lattice planes and boxes of zero extent), every sampled point's `fbm3` lies
  inside the box's `fbm3_range`.
- The range is useful, not just sound: over a box one cell wide it is narrower
  than the unit interval by a margin the test states, so a regression to a
  lattice-corner bound fails.
- `cargo test -p tesserae-core` and `./scripts/cargo-ci` pass.

## Comments
