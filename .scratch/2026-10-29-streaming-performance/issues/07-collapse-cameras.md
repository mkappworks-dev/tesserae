# 07: One depth buffer instead of six cameras

Status: ready-for-agent
Blocked by: 01, 02

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-cameras`.

## What to build

Every coarse level has its own camera today, each clearing depth, so finer
levels paint over coarser ones regardless of depth and every coarse pixel under
finer terrain is drawn and then overdrawn. With 02's coverage, coarse fragments
where a finer level is drawn are already discarded, so one depth buffer is
correct.

- Check depth precision of Bevy's reverse-Z out to `lod_distance_m` (3000 m in
  the demo, 4096 m by default) from the Player's near plane, with a capture of
  distant ridges.
- Draw every level with the Player camera into one depth buffer. If precision
  forces a second, coarse camera, it must not clear depth. Level 0 stays the
  only shadow caster; fog stays identical.
- Accept the one visual change: a coarse Chunk not yet refined can now hide
  finer terrain behind it.
- Amend 02's ADR.

## Acceptance criteria

- [ ] Render CPU or GPU time passes the spec's effect gate, per 01's
      attribution.
- [ ] No depth artefacts in review captures at 1 km and 2.5 km.
- [ ] Zero rule-hole frames still, in every fixture.

## Validation and authorization

Meets `spawn_camera` in `client/chunk_view.rs` and `client/camera.rs` as #29
left it. Local work only; no push or PR without approval.
