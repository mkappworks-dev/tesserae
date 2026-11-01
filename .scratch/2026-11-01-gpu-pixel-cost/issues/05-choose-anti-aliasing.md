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

## Validation and authorization

frame-profiling.md. Image quality is your call on captures. Local work only;
no push or PR without approval.
