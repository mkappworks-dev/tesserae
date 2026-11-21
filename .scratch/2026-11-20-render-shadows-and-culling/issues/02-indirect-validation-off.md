# 02: Indirect validation off in release

Status: done
Blocked by: 01

Spec: [Shadows and culling](../spec.md).

## What to build

wgpu validates every indirect draw in release on macOS, because Bevy drops
`VALIDATION_INDIRECT_CALL` only when the backends lack DX12 and its default
backends include DX12 everywhere. Each indirect render pass pays a compute pass,
a staging buffer and a copy.

- On macOS, the app's `WgpuSettings` backends are
  `Backends::from_env().unwrap_or(Backends::METAL)`, so `WGPU_BACKEND` still
  overrides, and release builds remove the flag themselves: Bevy decides it
  inside `WgpuSettings::default` from its own backends, so setting the
  backends alone keeps it. Debug builds and `WGPU_VALIDATION_INDIRECT_CALL=1`
  keep validation.
- One comment of why where it is set. A test that on macOS the backends are
  Metal and the flag is set in debug only, run in debug and in release.

## Acceptance criteria

- [x] Gated by 01's method, GPU frame as its target; numbers in Comments.
- [x] Captures against the commit before it differ no more than two runs of
      that commit do (the spec's image rule).
- [x] The game launches and draws, release and debug.

## Comments

Gate (2026-11-21), against 01, release `profiling` builds, load 5 to 6: passes.
The GPU frame fell beyond run spread in every fixture. Frame p50 fell in both
timing rounds except `sprint-flight`'s first, which rose inside 01's own spread
(3.28 to 4.19 ms); p95 stayed inside 01's spread.

| p50 ms (01 → 02) | GPU frame | frame, round 1 | frame, round 2 |
| --- | --- | --- | --- |
| flight | 4.32 → 4.11 | 3.65 → 3.43 | 4.29 → 3.90 |
| active | 4.85 → 4.62 | 4.09 → 3.97 | 4.09 → 3.89 |
| transparency | 4.96 → 4.54 | 4.18 → 4.13 | 4.20 → 3.94 |
| stationary | 4.74 → 4.26 | 3.49 → 3.27 | 3.80 → 3.50 |
| sprint-flight | 3.80 → 3.61 | 3.49 → 3.81 | 4.54 → 4.21 |

The Player camera's opaque pass fell 0.17 → 0.13 ms in every fixture. 01's
markers add about 1 ms to every GPU frame here (see 01), the same for both.

Captures: 02 against 01 differ by 0 to 58 pixels, as `origin/main`'s own runs
do (up to 58 in `stationary`, 32 in `transparency`). A debug build launches and
draws the same image (45 pixels off release).
