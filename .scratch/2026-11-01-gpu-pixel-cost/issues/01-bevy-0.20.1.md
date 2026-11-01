# 01: Move to Bevy 0.20.1

Status: ready-for-agent
Blocked by: none

Spec: [GPU cost per pixel](../spec.md).
PR boundary: this branch, `tech/gpu-pixel-cost`, with the spec.

## What to build

Move the workspace from Bevy 0.20.0 to 0.20.1, the latest release
(2026-10-28), before any measurement, so every series in this spec runs on one
Bevy.

- Raise the `bevy` requirement to 0.20.1 and update the lockfile for Bevy's
  crates only. wgpu stays 30.0.1, and so does our direct `wgpu` pin, which
  must match Bevy's.
- 0.20.1 requires `ruzstd ^0.9`, which resolves to 0.9.1, the fixed version
  of RUSTSEC-2026-0344. Delete that advisory's `ignore` entry and its comment
  from `deny.toml`, and close the advisory's ticket in
  `.scratch/2026-10-13-ruzstd-advisory/` with a comment and `Status: done`.
- Of 0.20.1's ten commits, two touch rendering: a fix to `UninitBufferVec`'s
  entry size and a removed cyclic shader import in `bevy_render`. Neither is
  expected to change our output; the captures below check it.

## Acceptance criteria

- [ ] `Cargo.lock` has Bevy 0.20.1, wgpu 30.0.1 and `ruzstd` 0.9.1 or later;
      `cargo tree -i ruzstd` shows no 0.8.
- [ ] `deny.toml` has no RUSTSEC-2026-0344 entry and `./scripts/cargo-ci`
      passes (`RUSTFLAGS="-D warnings"`, workspace form).
- [ ] Offscreen captures of `stationary` and the far view are pixel-identical
      to the tip's, or each difference is explained.
- [ ] One offscreen 1080p series, three interleaved rounds, `stationary` and
      `sprint-flight`, against the tip binary `df93f3b`: no regression beyond
      the run ranges. Recorded here.

## Validation and authorization

Builds and runs through `/tmp/claude-streaming-impl/heavy.sh`. Artifacts in
`/tmp/claude-gpu-pixel-cost/`. Local commits only; no push or PR without
approval.
