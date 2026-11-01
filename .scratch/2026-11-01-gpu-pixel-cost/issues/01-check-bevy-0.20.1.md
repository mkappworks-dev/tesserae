# 01: Check Bevy 0.20.1 against the tip

Status: ready-for-agent
Blocked by: none

Spec: [GPU cost per pixel](../spec.md).
PR boundary: this branch, `tech/gpu-pixel-cost`, with the spec.

## What to build

`main` already runs Bevy 0.20.1 (#49), with wgpu 30.0.1 and `ruzstd` 0.9.1,
and the RUSTSEC-2026-0344 ignore and its ticket are gone. What is left is the
check that the spec's baseline holds, before any measurement:

- Raise the `bevy` requirement in `crates/tesserae/Cargo.toml` from 0.20.0 to
  0.20.1, so the manifest says what the lockfile resolves.
- Of 0.20.1's ten commits, two touch rendering: a fix to `UninitBufferVec`'s
  entry size and a removed cyclic shader import in `bevy_render`. Neither is
  expected to change our output; the captures below check it.

## Acceptance criteria

- [ ] `./scripts/cargo-ci` passes (`RUSTFLAGS="-D warnings"`, workspace form).
- [ ] Offscreen captures of `stationary` and the far view (lift 90 m, pitch
      -5) are pixel-identical to the tip's `df93f3b`, or each difference is
      explained. Taken with host load under about 3, once streaming has
      settled: a capture taken mid-stream differs from itself.
- [ ] One offscreen 1080p series, three interleaved rounds, `stationary` and
      `sprint-flight`, against the tip binary `df93f3b`: no regression beyond
      the run ranges. Recorded here, with the host load of each run.

## Notes from the first attempt (2026-11-01)

Taken on the branch before the rebase, with the other session building: load
6.9 to 19.8, p50 of one binary ranging 1.6 to 3.3 ms, and `stationary` round 3
captures differing even between two runs of the tip. Not usable as evidence.
Rule and fill holes were zero in all 12 runs. Rerun with the host idle.

## Validation and authorization

Builds and runs through `/tmp/claude-streaming-impl/heavy.sh`. Artifacts in
`/tmp/claude-gpu-pixel-cost/`. Local commits only; no push or PR without
approval.
