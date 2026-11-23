# 01: Baseline captures

Status: ready-for-agent

Spec: [Texture shimmer and saturation](../spec.md).

## What to build

Captures on `main`, offscreen (`TESSERAE_SCREENSHOT_OFFSCREEN=1`), Terra, Eyes
with a fractional part:

- Near: terrain a few metres away, textured faces filling the frame.
- Far: a pitch that shows level 0 running into the coarse levels.
- Shimmer: two captures a fraction of a metre apart at the far Eye; their
  difference image shows the texel crawl.
- Cave: a space lit only by block light.

Record the Eyes and variables in Comments so later tickets repeat them exactly.
Images stay in one `/tmp` folder and are never committed.

## Acceptance

- [ ] Four views and the shimmer pair captured, Eyes recorded.
- [ ] The user has looked at them and said which parts read too saturated.
