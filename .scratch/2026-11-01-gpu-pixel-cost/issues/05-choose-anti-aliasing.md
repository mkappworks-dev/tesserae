# 05: Choose the anti-aliasing

Status: ready-for-agent
Blocked by: 02

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-anti-aliasing`.
Conditional: run only if 02's MSAA-off switch passes the effect gate;
otherwise `wontfix` with the numbers.

## What to build

The Player camera has Bevy's default 4x MSAA, never chosen. Choose it.

- Measure offscreen at 1440p, three rounds: MSAA 4x, MSAA 2x, MSAA off with
  FXAA, MSAA off with SMAA. If 06 has landed, also at its chosen scale.
- Captures of each for you: `stationary`, the far view, a near edge of
  leaves and of water, all from one `/tmp` folder.
- You choose. The choice is set explicitly on the Player camera, so a Bevy
  default can never change it again, and recorded in DESIGN.md's rendering
  decisions.

## Acceptance criteria

- [ ] Each option's frame and GPU time recorded here.
- [ ] Your choice recorded here, with the captures it was made on.
- [ ] Zero rule-hole frames in every fixture; offscreen 1080p does not
      regress against 02's baseline unless you accept it.

## Decision (2026-11-02): go ahead

02's MSAA-off switch saved 1.2 ms of 6.8 ms at 1440p, so the condition is
met. Approved with you, to be done in a new session. Start by rerunning the
`msaa-off` and `level0-only` switches on 03's binary (`bin-03`), offscreen
1440p, three rounds, so the gains are post-03 numbers and not 02's. 03 left
the presented p95 at 6.1 to 6.7 ms in passing runs against 6.94 ms, so the
point is headroom. You pick the anti-aliasing on captures.

## Validation and authorization

frame-profiling.md. Image quality is your call on captures. Local work only;
no push or PR without approval.
