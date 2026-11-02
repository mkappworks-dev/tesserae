# 06: Render scale with upscaling

Status: ready-for-agent
Blocked by: 02

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-render-scale`.

## What to build

A Client setting that draws the World at a fraction of the window on each axis
and upscales it, with UI at full resolution.

- Choices 1.0, 0.85, 0.75 and 0.67, in the settings menu and the settings
  file. Default 1.0 until you choose otherwise.
- Pick the mechanism by what keeps working: Bevy's
  `MainPassResolutionOverride`, which its DLSS path uses, or the World camera
  drawing into a scaled image that a full-resolution pass upscales. Distance
  fog, tonemapping, the underwater fog and the screenshot capture path must
  behave as at 1.0; record why the other was rejected.
- Bilinear upscale, with and without Bevy's contrast-adaptive sharpening;
  the captures decide whether sharpening ships.
- Changing the setting takes effect without restarting the World session.
- The runner records the render scale in each run's provenance, read from
  the engine, so scaled and unscaled runs are never pooled.

## Acceptance criteria

- [ ] Each scale's frame and GPU time offscreen at 1440p, three rounds.
- [ ] Captures at every scale, with and without sharpening, offscreen at
      1440p: `stationary`, the far view, the hotbar and a menu open. UI is
      identical across scales.
- [ ] A presented series by you at the scale you lean to (command written
      here when the binary exists, output folder right after the script name).
- [ ] Your choice of default recorded here.
- [ ] At 1.0 the image is pixel-identical to before; zero rule-hole frames in
      every fixture at every scale.

## Decision (2026-11-02): go ahead

The spec wants this setting whatever the target (user stories 6 to 8): its
value is larger displays and weaker Macs, not this one at 1440p. Approved
with you, to be done in a new session. Default 1.0 until you choose
otherwise on captures.

## Validation and authorization

frame-profiling.md. Image quality is your call on captures. Local work only;
no push or PR without approval.
