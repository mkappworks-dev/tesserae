# 08: Explain the two presented timing states

Status: ready-for-human
Blocked by: 07

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-present-waits`, cut from `tech/gpu-targets`.

## Decision (2026-11-02)

Approved by you after 07: investigate the roughly 4.4 to 8.4 ms presented
frame step, then repeat the quiet-host 1080p comparison. Keep SMAA and 100%
render scale. No present-mode or quality default changes are authorized.

## What to build

- Add opt-in CPU wall intervals enclosing window preparation/acquisition,
  the render graph, and renderer finalization after the graph. These are
  attribution intervals, not exact GPU execution or display latency.
- Record host load, AC/battery and available thermal observations before
  and after each run; record the window monitor's OS-reported refresh rate.
- Measure GPU attribution separately from timing, as the markers themselves
  cost too much for a timing baseline.
- Ask you to run the presented diagnostic on the unlocked Mac, focused and
  unobstructed. The agent stays idle until you say done.
- Repeat all eight 1080p fixtures against 02 on a quiet host, three matched
  interleaved rounds, holes on and GPU markers off. Retain every sample.

## Acceptance criteria

- [x] The new intervals and host observations are verified on a native run.
- [ ] Presented fast and slow states attributed to measured intervals, with
      uncertainty retained wherever an underlying wait is unavailable.
- [ ] Controlled timing repeats establish whether p95 <= 6.9 ms holds in
      all four presented fixtures at SMAA and 100% scale.
- [ ] The quiet-host 1080p median and p99 comparison recorded, or explicit
      evidence that the host cannot provide an uncontended comparison.
- [ ] Zero rule-hole frames; no `no ground` in accepted series.

## Implementation and next measurement

`--instruments waits` installs five exclusive boundary markers only when
requested. Exported frame IDs join the three intervals to the main samples;
missing or reversed boundaries stay unavailable. The normal timing path
has no interval markers. The runner summarizes every render `_ms` column,
retains host observations even on engine failure, and audits focus counts
and `no ground`. Monitor metadata uses the window's `OnMonitor` relation,
not the desktop's primary display.

Checks: six Rust profiler tests and ten Python runner tests pass; profiling
Clippy passes with warnings denied. The release profiling build passes.
Initial native offscreen smoke (before grouping the monitor queries to
satisfy Clippy), `/tmp/claude-gpu-pixel-cost/smoke08`: 7,744 render rows,
all three intervals populated on every row, complete fixture, zero holes,
no `no ground`. Host load 4.09 before, 3.75 after, AC attached; thermal
query reported no recorded warning/status. A hidden offscreen window
reported the internal monitor at 120,000 millihertz; this does not identify
the external monitor used by your presented runs.

The final binary is `/tmp/claude-gpu-pixel-cost/bin-08`. Before presenting,
its updated runner passed three stationary offscreen on/off rounds at
1440p in `/tmp/claude-gpu-pixel-cost/offscreen-waits-08`. All six processes
exited successfully; the three logging-on windows were complete with zero
rule-hole and fill-hole frames, and no log had `no ground`. All 22,932
render rows had all three intervals. Before/after host observations were
retained for every run, load 5.08 to 7.64; this validates the
instrumentation, not uncontended performance. Binary SHA-256:
`f344381c5cf62f1b0a55f70c80e21d6ecaab4b4fcfd1ba1f39da698938e0459a`.

The next presented series is attribution, not the performance gate. Use
`stationary` and `sprint-flight`, three rounds, to catch both timing states
with less time at the Mac. GPU markers stay off. Keep the window on the
144 Hz monitor, focused and unobstructed, with other busy applications
closed if possible. The agent stays idle throughout, until you say done.
Expected duration: about seven minutes. Paste as one command:

```sh
cd /Users/mk/Downloads/app/Tesserae/tesserae/.worktrees/tech+gpu-pixel-cost \
&& /tmp/claude-streaming-impl/heavy.sh scripts/profile-frames \
  /tmp/claude-gpu-pixel-cost/presented-waits-08 \
  --binary /tmp/claude-gpu-pixel-cost/bin-08 \
  --profile release --features profiling \
  --game /Users/mk/Downloads/app/Tesserae/tesserae/.worktrees/tech+gpu-pixel-cost/games/demo/dist/mods \
  --presented --render-scale 100 --instruments holes waits \
  --fixtures stationary sprint-flight
```

After attribution, collect GPU observations separately if needed, then
repeat the four-fixture presented timing gate without `waits` or `gpu`.
The 1080p matched repeat is prepared in
`/tmp/claude-gpu-pixel-cost/plan08-1080.json`, using `abx08.py` (a local copy
of the existing interleaved helper with per-run host observations). It
compares `bin-08` against `bin-02`, eight fixtures and three rounds, holes
on, no waits or GPU markers. Run through heavy.sh only when no build or
presented series is active and the host is quiet; do not silently discard
contended tails. Pending measurements keep this ticket open.

## Validation and authorization

frame-profiling.md. All heavy work goes through the shared heavy.sh lock.
Captures and raw artifacts stay under /tmp/claude-gpu-pixel-cost. Presented
runs need you at the Mac. Local work only; no push or PR without approval.
