# 05: Shared placeholders in size buckets (item 4)

Status: done
Blocked by: 01

Spec: [Shadows and culling](../spec.md).

## What to build

On Metal each mesh asset is its own indirect draw in every pass and cascade,
culled or not, and every Chunk layer has its own placeholder. Layers that share
a placeholder share a draw, with one instance each.

- Bucket sizes grow by 2^(1/4) quads. Before any GPU run, take the fixtures'
  layer quad counts and report the average padding and the bucket count; adjust
  the ratio there if they surprise.
- One render-world placeholder per bucket, built once: two-byte indices up to
  16,384 quads, four past them. A layer's entity takes its bucket's handle.
- The layer's header gains its quad count (`HEADER_WORDS` 3 to 4, a
  `HEADER_QUADS` def), and the vertex shader degenerates every quad past it.
- Opaque and cutout are binned phases and collapse to one draw per bucket.
  Blended layers are a sorted phase, drawn one by one: they share buckets for
  the memory only.
- ADR-0034's placeholder section and its "each layer has its own placeholder"
  consequence are amended, with the numbers.
- Launch the game after the change.

## Acceptance criteria

- [x] Tests: the bucket for a quad count; the expansion test draws a layer
      through a larger bucket and finds the padding degenerate.
- [x] Gated by 01's method: draws per frame (01's count), GPU frame and the
      shadow span, held mesh bytes and upload per frame; numbers in Comments.
- [x] Captures against the commit before it differ no more than two runs of
      that commit do (the spec's image rule).

## Comments

2026-11-20: bucket ratio checked before any GPU run. Layer quad counts logged
as they landed in two Terra captures (base build, `dev`, offscreen; the default
spawn and an ocean Eye), every level: 6,148 layers, 9.9 million quads, largest
23,419 (cutout), median 300.

| Ratio   | Padding (vertex-weighted) | Mean per layer | Buckets used |
| ------- | ------------------------- | -------------- | ------------ |
| 2^(1/8) | 4.3%                      | 3.4%           | 93           |
| 2^(1/4) | 9.1%                      | 7.5%           | 53           |
| 2^(1/2) | 20.2%                     | 16.4%          | 29           |

At 2^(1/4): opaque 9.9% in 43 buckets, cutout 8.8% in 47, blended 6.2% in 19;
level 0's opaque and cutout 9.1% in 50. Cutout holds 73% of the quads. No
surprise against the spec's 9%, so the ratio stays. Buckets are 1 to 6 one by
one, then grow by the ratio rounded up, with one ending at exactly 16,384 so
two-byte indices reach as far as before.

The camera's collision cache was keyed by the placeholder's asset id, which a
re-meshed layer now keeps; it is keyed by entity and the quads' change tick.

Captures, before (7833eaca) and after, offscreen 1920x1080, `stationary`,
`transparency` and `hidden-hill` with profile-frames' settings, twice each:
before against after differs in 0.0001-0.11% of pixels, base against base in
0.03-0.09%; every differing blob is a growing crop or the minimap, the same
places base against base differs.

Gate (2026-11-21), against 04, release `profiling` builds, load 5 to 6: passes
on draws, held bytes and upload; the GPU frame and frame time do not move.

- Draws a frame, Player camera: 241 → 123 in `stationary`, 267 → 127 in
  `flight`. Cascades 0 to 3: 3, 7, 28, 80 → 2, 4, 16, 31.
- Held Chunk mesh bytes: 24.1 → 11.0 MB (3.8 MB of it placeholders).
- Upload p95: 0.082 → 0.043 ms in `active`, about half in every fixture.
- GPU frame: flat in `stationary` (2.97 → 2.99 ms), +0.06 to +0.1 ms in
  `flight` and `sprint-flight`. Four of six `active` and `transparency` runs
  charged 01's marker stall (see 01) to the Player camera, so that series
  cannot rank them.
- Frame time, 03-04-05-05-04-03 back to back, timing only, pooled p50 for 04 →
  05: 3.02 → 3.01 ms `flight`, 3.27 → 3.25 `active`, 3.17 → 3.20
  `transparency`, 2.72 → 2.70 `stationary`. No worse.

04 was reverted after this gate, so 05 now sits on 03 (`cc926aa0`): its build
launches and draws the same 123 draws at a 2.98 to 3.01 ms GPU frame, and five
of six `stationary` captures are pixel-identical to 03's; the sixth, the first
launch of a fresh build, differs by 393 canopy pixels from 05's other runs as
much as from 03.
