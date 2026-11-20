# 02: Indirect validation off in release

Status: ready-for-agent
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

- [ ] Gated by 01's method, GPU frame as its target; numbers in Comments.
- [ ] Pixel-identical captures against the commit before it.
- [ ] The game launches and draws, release and debug.
