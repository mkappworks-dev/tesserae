# 01: Say why connectivity stays profiling-only

Status: ready-for-agent
Blocked by: none, once you confirm the spec's `wontfix`

Spec: [defer unreachable meshing](../spec.md).

## What to build

The next reader who finds Chunk connectivity computed under `dev` and
`profiling` and never used in release will try to put it to work. Leave the
answer where they will find it:

- One line of why on the Server's profiling-only connectivity, where
  `MeshState` keeps it: deferring the meshes of Chunks the Eye cannot reach
  would save about 1% of worker CPU, so release builds skip the work. The why
  and the number only; the history goes in the commit message.
- frame-profiling.md says the walks are counts "which nothing yet culls or
  reorders by". Drop "yet": 12 removed the cull and this spec declines the
  reorder.

No behaviour changes.

## Acceptance criteria

- [ ] The comment is one line and states the measured saving.
- [ ] frame-profiling.md no longer promises a later cull or reorder.
- [ ] `cargo fmt --all --check` and `npm run fmt:check` pass;
      `./scripts/cargo-ci` green, through `heavy.sh`.
