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
  --game /Users/mk/Downloads/app/Tesserae/tesserae/.worktrees/tech+streaming-performance-impl/games/demo/dist/mods \
  --presented --instruments gpu holes \
  --fixtures stationary sprint-flight
```

The output folder comes right after the script name because `--fixtures`
takes every argument after it.

## Acceptance criteria

- [ ] The offscreen size and the switches are in the runner and recorded in
      provenance; a normal build compiles none of the switches.
- [ ] GPU time per pass at 1080p and 1440p offscreen, and presented 1440p,
      recorded here.
- [ ] Each switch's effect measured offscreen at 1440p.
- [ ] The presented p95 tail attributed to GPU time or presentation.
- [ ] The offscreen 1440p timing baseline recorded, every fixture.
- [ ] A proposed order for 03 to 06, for your approval.

## Validation and authorization

frame-profiling.md: GPU series and timing series separate, executable hashes,
three interleaved rounds, nothing timed while a build runs, every run through
`/tmp/claude-streaming-impl/heavy.sh`. Local work only; no push or PR without
approval.
