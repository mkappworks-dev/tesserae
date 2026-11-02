# 05: Choose the anti-aliasing

Status: done
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

- [x] Each option's frame and GPU time recorded here.
- [x] Your choice recorded here, with the captures it was made on.
- [x] Zero rule-hole frames in every fixture; offscreen 1080p does not
      regress against 02's baseline unless you accept it.

## Decision (2026-11-02): go ahead

02's MSAA-off switch saved 1.2 ms of 6.8 ms at 1440p, so the condition is
met. Approved with you, to be done in a new session. Start by rerunning the
`msaa-off` and `level0-only` switches on 03's binary (`bin-03`), offscreen
1440p, three rounds, so the gains are post-03 numbers and not 02's. 03 left
the presented p95 at 6.1 to 6.7 ms in passing runs against 6.94 ms, so the
point is headroom. You pick the anti-aliasing on captures.

## Result (2026-11-02)

Post-03 switches first, `bin-03`, offscreen 2560 x 1440, three interleaved
rounds, host load 5.3 to 8.5. GPU frame with markers, median ms; wall p50
without markers:

| arm            | GPU `stationary` | GPU `sprint-flight` | p50 `stationary` |
| -------------- | ---------------- | ------------------- | ---------------- |
| default        | 4.74             | 4.98                | 4.47-4.66        |
| `msaa-off`     | 3.72             | 3.74                | 3.32-3.87        |
| `level0-only`  | 4.11             | 4.08                | 3.91-3.99        |

MSAA is still 1.0 to 1.2 ms after 03, a quarter of the GPU frame; the gate
holds. That switch understated it: the hand camera kept Bevy's default 4x
MSAA, and as a camera that draws over the World without clearing, it copies
the whole frame into a 4x target every frame (MSAA writeback). With both
cameras off the GPU frame is 3.45 to 3.65 ms (below), so the writeback was
0.1 to 0.3 ms. The hand camera now takes the Player camera's sample count.

`da8cc17`: the World cameras' anti-aliasing is a named choice
(`client/anti_alias.rs`), set explicitly so a Bevy default cannot change it,
and profiling builds gain `msaa-2`, `fxaa` and `smaa` beside `msaa-off`.
The capture pillar takes a material name for close-ups.

`bin-05` (`ae4e9880...`), offscreen 1440p, three interleaved rounds, host
load 4.5 to 8.7, zero rule-hole and fill-hole frames in all 66 runs:

| option  | GPU frame | p50 stationary | p50 sprint-flight | p95 stationary |
| ------- | --------- | -------------- | ----------------- | -------------- |
| MSAA 4x | 4.80      | 4.45-4.56      | 4.55-5.17         | 5.36-5.97      |
| MSAA 2x | 4.17      | 3.85-4.13      | 3.98-4.33         | 4.88-5.74      |
| SMAA    | 4.26      | 3.83-4.08      | 3.81-4.10         | 4.71-5.88      |
| FXAA    | 4.31      | 3.95-4.15      | 3.86-4.04         | 5.02-5.78      |
| off     | 3.45-3.65 | 3.07-3.55      | 3.46-3.81         | 4.48-4.94      |

`bin-03` in the same series: p50 4.46-4.54, the same as `bin-05` at 4x, and
their captures differ by 3,949 pixels against 1,699 between two rounds of
one binary: the hand camera change is free at 4x.

Captures: `stationary`, the far view, and close-ups of a leaves pillar and a
water pillar from the ground (`TESSERAE_FIXTURE_MOVEMENT=physics`, since the
flying Eye has nothing for a pillar to stand on), in
`/tmp/claude-gpu-pixel-cost/cap05`, `cap05d` and `aa05`; zoomed sheets in
`aa-sheets/`. MSAA 2x leaves faint steps on long diagonals; FXAA softens the
block textures; off is jagged; SMAA is close to 4x with crisp textures. The
demo's leaves are opaque, so no cutout edge was compared. SMAA and FXAA run
on the Player camera, so the held item, drawn after, is not filtered.

**Your choice (2026-11-02): SMAA**, on these captures. About 11% off the
1440p frame, ranges apart from 4x in both fixtures.

You then chose to make it a setting (2026-11-02): Settings > Presentation
offers SMAA (the default), MSAA 4x and Off, in the profile file as
`anti_aliasing`, applied to the Player and hand cameras in the frame it
changes. MSAA 2x and FXAA cost the same as SMAA and look worse, so they stay
profiling switches (`msaa-2`, `fxaa`, and `msaa-4` for the old default).
Recorded in DESIGN.md's settled decisions.

`bin-05s` (`eb89aab0...`, SMAA by default), offscreen, no GPU markers, three
interleaved rounds, host load 5.4 to 10.6. Zero rule-hole and fill-hole
frames in all 36 runs:

| size  | fixture       | p50 `bin-03` | p50 SMAA  | p95 SMAA   |
| ----- | ------------- | ------------ | --------- | ---------- |
| 1440p | stationary    | 4.41-4.50    | 3.68-3.82 | 4.83-5.17  |
| 1440p | flight        | 4.80-5.16    | 4.06-4.26 | 5.23-6.07  |
| 1440p | sprint-flight | 4.42-4.56    | 3.72-4.04 | 5.53-6.39  |
| 1440p | sprint-walk   | 4.43-4.46    | 3.90-4.14 | 5.48-6.43  |
| 1440p | dive          | 3.72-3.79    | 3.16-3.26 | 4.53-6.10  |
| 1440p | teleport      | 4.51-4.64    | 3.97-4.36 | 5.47-10.97 |
| 1440p | transparency  | 5.60-5.79    | 4.83-5.34 | 5.77-8.73  |
| 1440p | active        | 4.43-4.51    | 3.78-4.26 | 4.87-7.42  |
| 1080p | stationary    | 3.31-3.40    | 3.08-3.48 | 4.26-10.32 |
| 1080p | sprint-flight | 3.41-3.66    | 3.11-3.36 | 5.03-5.85  |

The 1440p `bin-03` column is 03's series, not this one; the 1080p one ran
interleaved here. 1080p does not regress: the medians overlap at
`stationary` and SMAA is lower at `sprint-flight`. The p95 tails over
6.5 ms are four runs during a burst of host activity (load 6.4 to 9.7, 7 to
15 frames over 16.7 ms each): `stationary` 1080p round 1 and `active`,
`transparency` and `teleport` round 2. Contended, not rerun here; 07
measures every fixture again.

## Validation and authorization

frame-profiling.md. Image quality is your call on captures. Local work only;
no push or PR without approval.
