# 14: Benchmark, digests and accepting version 3

Status: ready-for-agent

Blocked by: 13

Spec: [Phase 17](../spec.md).

## What to build

Interleaved CPU-time A/B against `main`, recorded version 3 digests, and the bump: `GENERATOR_VERSION = 3`, the save accepts 1, 2 and 3, and the save tests that hard-code the set are updated.

## Acceptance criteria

- [ ] Surface Chunks within about 3x of version 2; early-outs unregressed.
- [ ] Versions 1 and 2 digests unchanged.
- [ ] A new World is version 3; a version 2 World opens as version 2.
