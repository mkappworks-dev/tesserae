# 02: Attribute GPU time at 1440p

Status: ready-for-agent
Blocked by: 01

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-attribution`.

## What to build

Name where the GPU frame goes at 2560 x 1440 before anything is cut.

- **Runner, offscreen size.** `scripts/profile-frames` gains a size option
  for its offscreen target, default 1920 x 1080, recorded in the provenance
  as the physical target. The engine already reads the size from
  `TESSERAE_SCREENSHOT_WIDTH` and `_HEIGHT`.
- **Runner, diagnostic switches.** The runner strips every `TESSERAE_`
  variable from the environment, so it gains an option naming switches to
  set, recorded in the provenance and each run's `run.json`.
- **Diagnostic switches** in `profiling` builds only, default off, each
  changing one thing: no coverage discard (the coverage test compiled out;
  the image is wrong), level 0 only, MSAA off on the Player camera, no sun
  shadows. A normal build has none of them.
- **Series**, release `profiling`, `--instruments gpu holes`:
  - offscreen 1920 x 1080 and 2560 x 1440, `stationary` and `sprint-flight`,
    three interleaved rounds, at the default;
  - offscreen 2560 x 1440, the same, once per switch;
  - presented, run by you, at the default (command below).
- **A timing baseline without GPU markers**, offscreen 2560 x 1440, every
  fixture, `--instruments holes`: the before side for 03 to 06.
- **Checkpoint**, appended here:
  - each pass's GPU time at 1080p and 1440p, and which scale with pixels;
  - each switch's effect on GPU frame time and frame p50/p95;
  - presented: in frames over the p50, how much of the wall frame the GPU
    frame span covers, so the p95 tail is named GPU or presentation;
  - which of 03, 05 and 06 to run, in what order, and any new ticket for a
    dominant cost the spec did not foresee (shadows, presentation, post).

## Presented run: your step at the unlocked Mac

After the binary is built, with the engine window on the 144 Hz display and
nothing else running. Paste as one command; it takes about 8 minutes, and the
window must stay unobstructed until the runner prints its summary.

```sh
cd /Users/mk/Downloads/app/Tesserae/tesserae/.worktrees/tech+gpu-pixel-cost \
&& /tmp/claude-streaming-impl/heavy.sh scripts/profile-frames \
  /tmp/claude-gpu-pixel-cost/presented-gpu-02 \
  --binary /tmp/claude-gpu-pixel-cost/bin-02 \
  --profile release --features profiling \
  --game /Users/mk/Downloads/app/Tesserae/tesserae/.worktrees/tech+gpu-pixel-cost/games/demo/dist/mods \
  --presented --instruments gpu holes \
  --fixtures stationary sprint-flight
```

The output folder comes right after the script name because `--fixtures`
takes every argument after it.

## Acceptance criteria

- [x] The offscreen size and the switches are in the runner and recorded in
      provenance; a normal build compiles none of the switches.
- [ ] GPU time per pass at 1080p and 1440p offscreen, and presented 1440p,
      recorded here. (Offscreen: GPU frame totals only; see the checkpoint.
      Presented: waits for your run.)
- [x] Each switch's effect measured offscreen at 1440p.
- [ ] The presented p95 tail attributed to GPU time or presentation.
- [x] The offscreen 1440p timing baseline recorded, every fixture.
- [x] A proposed order for 03 to 06, for your approval.

## Checkpoint (2026-11-02), offscreen; the presented run is still to do

Release `profiling` binary `bin-02` (sha256 `86581972...`), the demo Game
built from this worktree, `--instruments gpu` markers on in the attribution
series and off in the baseline, host load 3 to 7.7 (not idle: the effects
below are 5 to 10 times the 0.2 ms spread between rounds). Zero rule-hole and
fill-hole frames in all 36 attribution runs and all 24 baseline runs.

GPU frame, median ms, three rounds, `stationary` (`sprint-flight` agrees to
0.1 ms):

| arm                          | GPU frame | wall p50    | wall p95    |
| ---------------------------- | --------- | ----------- | ----------- |
| 1080p default                | 4.76      | 4.6 to 4.8  | 6.1         |
| 1440p default                | 6.77      | 6.6 to 6.9  | 7.9 to 8.2  |
| 1440p, no coverage discard   | 4.38      | 4.4 to 4.8  | 5.6 to 6.7  |
| 1440p, level 0 only          | 4.35      | 4.3 to 4.4  | 5.5 to 5.8  |
| 1440p, MSAA off              | 5.60      | 5.5 to 5.6  | 6.8 to 7.3  |
| 1440p, no sun shadows        | 5.74      | 5.7         | 8.9 to 10.2 |

- The frame is GPU-bound: GPU frame equals wall p50 to 0.1 ms.
- 1440p has 1.78 times the pixels of 1080p and its GPU frame is 1.42 times
  longer (2.0 ms more): most of the frame scales with pixels.
- **The coverage discard is 2.4 ms (35%) of the 1440p GPU frame.** Drawing
  every level with the test compiled out recovers the same time as drawing
  level 0 alone, so the cost is the discard itself, not the overlapping
  coarse geometry being shaded. That is the spec's hypothesis, confirmed.
- MSAA 4x is 1.2 ms. The sun's shadow maps are 1.0 ms of GPU frame, but the
  no-shadows arm has a worse p95 (8.9 to 10.2 ms) that I cannot explain.
- **Per-pass split: inconclusive.** The marker segments (prepass, opaque,
  transparent, post) are 0.02 to 1.0 ms each and sum to far less than the
  frame; most of it falls between cameras and is charged to a side by the
  marker rule in frame-profiling.md. No pass is named from the markers; the
  switches are the attribution.

Offscreen 2560 x 1440 timing baseline (no markers, `--instruments holes`,
the before side for 03 to 06), three rounds, ms:

| fixture       | p50 range   | p95 range   | p99 range    |
| ------------- | ----------- | ----------- | ------------ |
| stationary    | 6.38 to 6.65 | 7.39 to 7.66 | 8.05 to 8.51 |
| flight        | 6.73 to 6.93 | 7.93 to 8.11 | 8.65 to 8.83 |
| sprint-flight | 6.52 to 6.82 | 7.93 to 8.44 | 8.72 to 9.84 |
| sprint-walk   | 6.55 to 6.76 | 7.83 to 8.43 | 8.64 to 9.57 |
| dive          | 4.82 to 4.94 | 6.77 to 6.89 | 7.52 to 7.84 |
| teleport      | 6.61 to 6.97 | 7.83 to 8.00 | 8.79 to 9.09 |
| transparency  | 7.53 to 8.10 | 8.93 to 9.46 | 9.74 to 10.48 |
| active        | 6.70 to 6.94 | 7.61 to 8.11 | 8.33 to 8.81 |

Offscreen 1440p already misses 6.9 ms at p95 in every fixture but `dive`
(6.8 to 6.9), so the target needs cuts, not only presentation fixes.

Proposed order, for your approval:

1. **03, discard only where the coverage rule cuts.** The largest single cost
   (2.4 ms) and an exact cut. Expected, if it recovers most of it: 1440p
   p95 about 5.6 to 6.7 ms offscreen, inside the target.
2. **05, anti-aliasing.** 1.2 ms of MSAA is next; it changes the image, so it
   needs your pick on captures.
3. **04 and 06 only if needed** after 03 and 05 and the presented run: 04 if
   boundary Chunks still cost, 06 if the presented p95 is still over.
4. **A new ticket for the sun's shadows** (1.0 ms) only if you want it; the
   unexplained p95 rise needs its own look first. Not proposed to start.

The presented run (your step) names whether the p95 tail is GPU time or
presentation, and it can change this order.

## Validation and authorization

frame-profiling.md: GPU series and timing series separate, executable hashes,
three interleaved rounds, nothing timed while a build runs, every run through
`/tmp/claude-streaming-impl/heavy.sh`. Local work only; no push or PR without
approval.
