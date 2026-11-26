# 13: Wind and sea waves

Status: ready-for-agent
Blocked by: 10

Spec: [Water that looks natural](../spec.md), Sea waves and Wind.

## What to build

Waves roll in from the open sea and break on the beach. The Server draws one
Wind per World from the seed, a direction and strength, and publishes it to the
Client when a World session starts. The shader sums four to eight Gerstner waves
around the Wind in the normals only, steepnesses summing to at most 1. Each
wave's angular frequency is fixed and a whole multiple of 2π/3600 s; its
wavenumber is solved per fragment from ω² = g k tanh(k D) over the filtered
depth, and its direction bends toward the shore along the depth's gradient.
Waves too short for a pixel are dropped. A breaker band steepens and foams
toward the shore. Lakes ripple with the Wind's strength, far less than the sea.

## Acceptance criteria

- [ ] Tests: the Wind is the same for the same seed and differs across seeds; it
      reaches the Client when a World session starts.
- [ ] The frequency unit test from 08 covers every wave.
- [ ] Coast captures over several seconds: waves from one direction, shortening
      and turning toward the shore, foam breaking on the beach; shown to the
      user.
- [ ] No sparkle or crawl at distance in a moving pair.
