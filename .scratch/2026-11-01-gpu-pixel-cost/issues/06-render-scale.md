# 06: Render scale with upscaling

Status: done
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

- [x] Each scale's frame and GPU time offscreen at 1440p, three rounds.
- [x] Captures at every scale, with and without sharpening, offscreen at
      1440p: `stationary`, the far view, the hotbar and a menu open. UI is
      identical across scales.
- [ ] A presented series by you at the scale you lean to (command written
      here when the binary exists, output folder right after the script name).
- [x] Your choice of default recorded here.
- [x] At 1.0 the image is pixel-identical to before; zero rule-hole frames in
      every fixture at every scale.

## Decision (2026-11-02): go ahead

The spec wants this setting whatever the target (user stories 6 to 8): its
value is larger displays and weaker Macs, not this one at 1440p. Approved
with you, to be done in a new session. Default 1.0 until you choose
otherwise on captures.

## Result (2026-11-02)

`5d5cbd5`: the World cameras draw into an image a fraction of the output on
each axis, and a 2D camera of the output's own size upscales it and draws
the UI over it (`client/render_scale.rs`). At 100% none of this exists and
the World cameras draw to the output directly. The setting is in Settings >
Presentation (100, 85, 75, 67%) and in the profile file, and takes effect
without a new session. The runner's `--render-scale` and `--sharpen` set it
for a series, and each run's `run.json` records what the engine reports.

Mechanism: a scaled image target. `MainPassResolutionOverride` was rejected:
it narrows the main pass to a corner of full-size textures and leaves the
upscale to DLSS, so post-processing and the MSAA resolve would still run at
full size and the upscale would be ours to write inside Bevy's graph. Fog is
in the chunk shader and the underwater fog is per camera, both unaffected by
the target's size; tonemapping runs on the World camera, and the 2D camera
has none, so nothing is tonemapped twice. A capture renders the 2D camera
into the capture image. Sharpening is Bevy's contrast-adaptive sharpening on
the 2D camera, after the upscale and before the UI.

`bin-06` (`c32b988b...`, MSAA 4x as on `bin-03`), offscreen 2560 x 1440,
three interleaved rounds, host load 5.3 to 9.3, zero rule-hole and
fill-hole frames in all 80 runs:

| scale     | GPU frame | p50 stationary | p50 sprint-flight | p95 stationary |
| --------- | --------- | -------------- | ----------------- | -------------- |
| 100%      | 4.79      | 4.50-4.58      | 4.59-4.62         | 5.38-6.10      |
| 85%       | 4.12      | 3.87-3.93      | 3.95-4.23         | 4.99-5.27      |
| 75%       | 3.67      | 3.57-3.77      | 3.59-4.27         | 5.07-5.52      |
| 75% sharp | -         | 3.73-4.04      | 3.58-4.08         | 4.95-5.48      |
| 67%       | 3.30      | 3.34-3.46      | 3.53-3.75         | 4.80-4.95      |

At 100% the image is `bin-03`'s: 0 pixels differ in the far view, 13 (max
channel delta 3) in `teleport`, and 5,772 in `stationary` against 3,949 to
6,906 between rounds of one binary. The UI is identical at every scale: 0
pixels differ inside the hotbar slots and in the open menu. Captures in
`/tmp/claude-gpu-pixel-cost/cap06` and `sc06`, crops in `rs-crops/`. The
block textures soften at 85% and blur visibly at 75% and 67%; sharpening
restores much of their contrast for about 0.15 ms.

`bin-06s` (`e01c8cd3...`, sharpening on), every fixture once at 85, 75 and
67%: zero rule-hole and fill-hole frames in all 24 runs. At 100%, the same
holds through 07's series.

**Your choice (2026-11-02): default 100%, sharpening on below 100%.** At
100% the World draws to the window exactly as before, so the presented
series at your choice is 07's, at SMAA and 100%; the box above stays open
until it runs.

## Validation and authorization

frame-profiling.md. Image quality is your call on captures. Local work only;
no push or PR without approval.
