# 01: Check Bevy 0.20.1 against the tip

Status: done
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

- [x] `./scripts/cargo-ci` passes (`RUSTFLAGS="-D warnings"`, workspace form).
- [x] Offscreen captures of `stationary` and the far view (lift 90 m, pitch
      -5) are pixel-identical to the tip's `df93f3b`, or each difference is
      explained. Taken with host load under about 3, once streaming has
      settled: a capture taken mid-stream differs from itself.
- [x] One offscreen 1080p series, three interleaved rounds, `stationary` and
      `sprint-flight`, against the tip binary `df93f3b`: no regression beyond
      the run ranges. Recorded here, with the host load of each run.

## Result (2026-11-02)

`bevy = "0.20.1"` in `crates/tesserae/Cargo.toml`; `./scripts/cargo-ci` passes.
Release `profiling` binary sha256 `87ad75cc1f81e287...`, against the tip
`df93f3b`. Offscreen 1080p, three interleaved rounds, host load 3.0 to 5.5.

| fixture       | binary | p50 ms        | p95 ms        | rule / fill holes |
| ------------- | ------ | ------------- | ------------- | ----------------- |
| stationary    | tip    | 4.30 to 4.41  | 5.26 to 5.41  | 0 / 0             |
| stationary    | 0.20.1 | 4.34 to 4.46  | 5.36 to 6.05  | 0 / 0             |
| sprint-flight | tip    | 4.44 to 4.48  | 5.55 to 5.60  | 0 / 0             |
| sprint-flight | 0.20.1 | 4.45 to 4.46  | 5.55 to 5.62  | 0 / 0             |

Medians are inside each other's ranges. The one high p95 (6.05) is round 1 of
`stationary`, with 2 frames over 16.7 ms and a max of 19.1 ms; rounds 2 and 3
are 5.36 and 5.43. No consistent p99 change (5.9 to 6.9 on both).

Captures: pixel differences between the tip and 0.20.1 are 127 to 993 of
2,073,600 pixels in `stationary` and 357 to 603 in the far view (lift 90 m,
pitch -5). The tip differs from itself by about 3,200 and 899 pixels between
runs, so the difference is inside run-to-run noise. No fixed-view diff was
needed beyond that.

The first two attempts were void: the demo Game folder in the runner
(`tech+streaming-performance-impl`) no longer existed, so the World had no
ground ("no Game named rock, soil and surface" in the engine log). The demo
Game now comes from `scripts/build-guests` in this worktree. Check for that
line in `native.log` before trusting any series.

## Validation and authorization

Builds and runs through `/tmp/claude-streaming-impl/heavy.sh`. Artifacts in
`/tmp/claude-gpu-pixel-cost/`. Local commits only; no push or PR without
approval.
