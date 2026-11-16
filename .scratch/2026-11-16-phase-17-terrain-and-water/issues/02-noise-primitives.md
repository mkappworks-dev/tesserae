# 02: Noise primitives

Status: ready-for-agent

Blocked by: 01

Spec: [Phase 17](../spec.md).

## What to build

Ridged noise, a clamped domain-warp helper and a sound 2D range bound in `noise.rs`, beside the existing functions, which are unchanged.

## Acceptance criteria

- [ ] Old noise outputs are bit-identical.
- [ ] The range bound contains every sampled value in a property test.
- [ ] The warp displacement never exceeds its clamp.
