# 08: Explain the two presented timing states

Status: done
Blocked by: 07

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-present-waits`, cut from `tech/gpu-targets`.

## Accepted closure (2026-11-03)

You instructed that ticket 08 be marked done and the remaining checkbox
ticked, accepting the attribution limits already described. That checkbox
is closed by this explicit decision, not by an additional measurement.

Measured: the slow internal-display state is dominated by the enclosing
window-preparation interval; the focused external series passes the target
at verified geometry. Not measured: fast-state wait intervals paired with
the slow state under controlled matching conditions. The exact driver or
compositor cause is therefore unproven. Those limits remain in this record
and PR 50; no missing capture or causal proof is claimed.

The previously proposed paired capture and separate GPU attribution are
optional future research, not outstanding acceptance work for this ticket.

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
- [x] Presented fast and slow states attributed to measured intervals, with
      uncertainty retained wherever an underlying wait is unavailable.
- [x] Controlled timing repeats establish whether p95 <= 6.9 ms holds in
      all four presented fixtures at SMAA and 100% scale.
- [x] The quiet-host 1080p median and p99 comparison recorded, or explicit
      evidence that the host cannot provide an uncontended comparison.
- [x] Zero rule-hole frames; no `no ground` in accepted series.

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

## Presented attribution result (2026-11-02)

Your completed `presented-waits-08` series uses the final `bin-08`, SMAA,
100% scale, holes and waits on, GPU markers off. All six logging-on windows
are complete, with zero rule-hole and fill-hole frames; no `no ground`
appears in any of the twelve native logs. Stationary rounds 1 and 2 are
unfocused throughout; stationary round 3 and all sprint-flight rounds are
focused throughout. Host load before/after runs ranges 2.21 to 5.08, AC
attached; thermal queries have no recorded warning/status rather than a
measured temperature.

Per-run frame distributions, milliseconds, whole 22 to 32 s window:

| fixture | round | p50 | p95 | p99 | max | focus |
| ------- | ----- | --- | --- | --- | --- | ----- |
| stationary | 1 | 8.37 | 14.88 | 16.06 | 23.79 | no |
| stationary | 2 | 8.38 | 15.83 | 16.44 | 22.38 | no |
| stationary | 3 | 8.40 | 16.02 | 16.76 | 17.44 | yes |
| sprint-flight | 1 | 8.45 | 16.51 | 17.05 | 22.35 | yes |
| sprint-flight | 2 | 8.89 | 16.77 | 16.87 | 18.39 | yes |
| sprint-flight | 3 | 8.52 | 16.74 | 16.87 | 19.13 | yes |

CPU wall attribution intervals, p50 / p95 in ms. These are enclosing
intervals that can include scheduling and other systems, not isolated
calls. Percentiles are not added together.

| fixture | round | window prepare | render graph | finalization |
| ------- | ----- | -------------- | ------------ | ------------ |
| stationary | 1 | 5.422 / 11.225 | 1.215 / 1.325 | 0.021 / 0.031 |
| stationary | 2 | 5.356 / 12.564 | 1.222 / 1.416 | 0.023 / 0.033 |
| stationary | 3 | 5.312 / 12.800 | 1.229 / 1.548 | 0.023 / 0.038 |
| sprint-flight | 1 | 5.522 / 13.317 | 1.213 / 1.759 | 0.023 / 0.039 |
| sprint-flight | 2 | 5.669 / 14.041 | 1.233 / 1.787 | 0.024 / 0.044 |
| sprint-flight | 3 | 5.620 / 13.879 | 1.206 / 1.613 | 0.023 / 0.033 |

The slow state is dominated by window preparation, which encloses Bevy's
swapchain acquisition: 5.31 to 5.67 ms p50 and 11.23 to 14.04 ms p95. Its
summed duration is 68 to 74% of summed render-schedule wall duration over
each window. Same-render-frame Pearson correlation with render-schedule
wall time is 0.965 to 0.992. The graph stays at 1.21 to 1.23 ms p50 and
finalization (including the final submission and presentation scheduling)
at 0.021 to 0.024 ms. Render-thread CPU p50 is 1.31 to 1.35 ms. This locates
the slow region; it does not distinguish GPU backpressure from compositor
or driver pacing inside acquisition. A fast presented state was not
reproduced, so its transition is not fully explained yet.

Every logging-on run reports the same window monitor:
`Monitor #41057`, physical 3024 x 1964, OS-reported 120,000 millihertz.
This matches the internal monitor seen in the smoke check. The surface is
2560 x 1440 at scale 2 and changes from Fifo to Immediate at startup.
The 8.4 / 16.7 ms pattern is consistent with 120 Hz pacing, but these
intervals do not prove the underlying mechanism. This run is not evidence
of the external 144 Hz target. You confirmed that the windows were on the
internal display. No default has been changed. A profiling-only `--monitor-hz`
selector is being added
so every new process can open on the verified external monitor. The
external target check remains pending.

## Matched 1080p repeat (2026-11-02): target met

`quiet1080-08`, `bin-08` against 02, eight fixtures and three interleaved
rounds, native 1920 x 1080 offscreen, holes on, waits and GPU markers off.
All 48 processes completed with full fixture windows, readiness before
flight, fixture readiness, zero rule-hole and fill-hole frames, and no
`no ground` in any native log. Every active window had burning fuel.

The host began near load 2.1 on AC, without a competing build or benchmark;
Docker was about 7% CPU rather than the earlier sustained VM saturation.
Before/after load observations range 2.11 to 7.44 during the serial series.
This is lower background contention, not a claim of an idle machine.
Raw power and thermal observations are retained per run. No samples or
rounds were dropped. Isolated maxima remain visible in the table.

Ranges of per-run percentiles, milliseconds:

| arm | fixture | p50 | p95 | p99 | max |
| --- | ------- | --- | --- | --- | --- |
| 02 | stationary | 4.40-4.57 | 5.40-5.51 | 6.03-6.25 | 7.25-10.27 |
| 08 | stationary | 3.09-3.42 | 4.46-4.80 | 4.96-5.49 | 6.69-8.53 |
| 02 | flight | 4.84-4.87 | 5.98-6.11 | 6.68-7.01 | 10.30-13.55 |
| 08 | flight | 3.12-3.32 | 4.77-5.22 | 5.61-6.40 | 10.52-12.96 |
| 02 | sprint-flight | 4.67-4.72 | 5.89-6.70 | 7.12-9.32 | 11.14-30.04 |
| 08 | sprint-flight | 3.13-3.21 | 5.14-5.21 | 6.19-6.55 | 9.96-12.94 |
| 02 | sprint-walk | 4.60-4.97 | 5.60-7.61 | 6.30-9.81 | 10.39-34.77 |
| 08 | sprint-walk | 3.07-3.62 | 4.59-5.36 | 5.39-6.07 | 9.97-11.95 |
| 02 | dive | 3.12-3.54 | 4.77-4.90 | 5.87-6.03 | 9.86-29.57 |
| 08 | dive | 2.67-2.75 | 3.74-4.01 | 4.57-5.09 | 10.03-12.90 |
| 02 | teleport | 4.71-4.72 | 5.59-5.87 | 6.70-6.99 | 7.41-8.76 |
| 08 | teleport | 3.01-3.19 | 4.59-5.00 | 5.37-5.89 | 6.29-8.39 |
| 02 | transparency | 5.13-5.22 | 5.78-6.19 | 6.45-7.06 | 7.18-12.44 |
| 08 | transparency | 3.44-3.64 | 4.58-5.22 | 5.14-5.92 | 6.85-18.56 |
| 02 | active | 4.67-4.80 | 5.48-6.10 | 6.25-6.93 | 7.13-9.91 |
| 08 | active | 3.01-3.37 | 4.21-4.82 | 4.74-5.40 | 5.46-19.13 |

Every fixture's tip median range lies wholly below its baseline range;
so does every p99 range. The 1080p no-regression gate is met in this
repeat. Some isolated maxima are higher on the tip (notably active and
transparency); the target is median and consistent p99, not maximum.
This resolves 07's inconclusive 1080p tail check. The baseline uses MSAA
4x, the tip your approved SMAA, both at 100% scale.

## Corrected external-display check

A profiling-only `--monitor-hz 144` selects the unique monitor reporting
144 Hz within 1 Hz, moves the window there and sets the requested physical
size after arrival. It does not change monitor refresh or present mode.
Absent or ambiguous monitors fail; each process must confirm arrival
within five seconds. The runner checks that confirmation for logging-off
runs too, and checks final monitor and physical-size metadata when present.
Eight Rust profiler tests and eleven Python runner tests cover the new
selection, sizing and rejection paths. Clippy and the release build pass.
Real external placement remains to
be verified in the next presented run.

Use the normal timing instruments for the four-fixture target check,
without the serializing wait markers. With the external display connected
and set to 144 Hz, keep the engine focused and unobstructed throughout.
Expected duration: about fourteen minutes. The agent stays idle until you
say done. Binary: `/tmp/claude-gpu-pixel-cost/bin-08m`.

```sh
cd /Users/mk/Downloads/app/Tesserae/tesserae/.worktrees/tech+gpu-pixel-cost \
&& /tmp/claude-streaming-impl/heavy.sh scripts/profile-frames \
  /tmp/claude-gpu-pixel-cost/presented-external-08 \
  --binary /tmp/claude-gpu-pixel-cost/bin-08m \
  --profile release --features profiling \
  --game /Users/mk/Downloads/app/Tesserae/tesserae/.worktrees/tech+gpu-pixel-cost/games/demo/dist/mods \
  --presented --monitor-hz 144 --size 2560x1440 \
  --render-scale 100 --instruments holes \
  --fixtures stationary flight sprint-flight sprint-walk
```

The target remains open pending that run. The previous internal-display
attribution identifies a slow acquisition region, not a proven driver or
compositor cause and not a reproduction of both timing states.

## External window size rejection (2026-11-02)

`presented-external-08` stopped after its first logging-on run. Selection
worked: external `Monitor #10002`, physical 2560 x 1440, reported 144 Hz,
focused, SMAA, 100% scale. Backend logs show the surface briefly at
2560 x 1440, then macOS reduced it to 2560 x 1378; final metadata and both
World cameras agree. The exact-size guard correctly rejected it. The
assertion message incorrectly made this sound like monitor selection had
failed. This is not an accepted 1440p target run.

The runner now reports the actual content size in that rejection. Explicit
`--borderless` with `--monitor-hz` requests borderless fullscreen on the
selected monitor, with the requested target required to match its physical
size. Final metadata includes the window mode, checked by the runner.
This is a profiling-only presentation condition, recorded separately;
windowed and borderless measurements are not pooled. Normal window and
present-mode defaults are unchanged. Real OS placement and sizing still
need the next presented run; tests cover the requested mode, monitor,
physical sizing and the observed 1378-pixel rejection.

Repeat the previous four-fixture command with `--borderless`, the binary
`/tmp/claude-gpu-pixel-cost/bin-08b`, and fresh output folder
`/tmp/claude-gpu-pixel-cost/presented-external-08b`. All other arguments
stay the same. The first accepted run must report the external 144 Hz
monitor, 2560 x 1440 content, and borderless fullscreen. Agent idle until
you report completion.

## Borderless transition rejection (2026-11-02)

`presented-external-08b` also stopped after its first logging-on run. It
reached the external 144 Hz monitor and reported borderless fullscreen,
but the surface went from 1280 x 720 to 2560 x 1440 and immediately back to
1280 x 720. Final metadata and both World cameras agree on 1280 x 720;
focus was false. This is another rejected target run, not timing evidence.

The setup code requested fullscreen and physical resizing together, then
reported confirmation before observing either finish. Bevy propagates
mode before resolution, while winit's macOS fullscreen transition is
asynchronous. The simultaneous requests are a plausible source of the
reversion; the underlying OS cause is not proven from these logs.

Setup now stages the mode change separately. It waits two seconds for the
transition, allows one corrective resize if still needed, and requires
the correct monitor, mode and physical dimensions to remain stable for a
second before reporting confirmation. A ten-second startup deadline fails
with requested and observed dimensions, instead of continuing through a
fixture after premature confirmation. Unit tests emulate delayed arrival,
mode application and stale size, and ensure confirmation is withheld until
stable. Eight Rust profiler tests and twelve Python tests pass; native OS
transition validation remains pending.

Next binary: `/tmp/claude-gpu-pixel-cost/bin-08c`. Use the previous command
with fresh output `/tmp/claude-gpu-pixel-cost/presented-external-08c` and
that binary. Keep `--borderless --monitor-hz 144 --size 2560x1440`.

## Startup guard rejection (2026-11-02)

`presented-external-08c` failed the startup guard after about ten seconds,
before a fixture window. The observed size was 2560 x 1408, with the
borderless component set; the surface briefly reached 2560 x 1440 first.
This confirms that waiting alone did not fix the full-height content area.
No performance evidence is accepted from this attempt.

Bevy exposes macOS `fullsize_content_view` and `titlebar_transparent` as
window creation attributes; the former allows content behind the titlebar.
They were still at their defaults. Both are now enabled at creation only
for the explicit borderless profiler request. The remaining 32-pixel gap
suggests a titlebar/content-area issue, but native verification is pending;
these logs do not prove the exact Cocoa behavior.

A new `--monitor-probe` runs one short logging-on process and exits after
stable monitor/mode/size confirmation. It does not require a fixture window
or produce performance analysis. Use it before another full series:
`/tmp/claude-gpu-pixel-cost/bin-08d`, output
`/tmp/claude-gpu-pixel-cost/monitor-probe-08d`, with
`--presented --monitor-hz 144 --borderless --size 2560x1440 --monitor-probe`.
This probe still requires you at the Mac, and the agent stays idle until
you report completion. Prior failed folders are retained. Validation:
eight Rust tests, thirteen runner tests, Clippy and release build pass.
The probe runner test uses a mocked process; it is not OS validation.

## Native monitor probe passed (2026-11-02)

Your first `monitor-probe-08d` completed successfully in 6.26 seconds with
`bin-08d` (SHA-256 `941df4b2c836cdcf...`). Backend surface configuration,
final window metadata and both World cameras all report 2560 x 1440.
The selected external monitor reports 144,000 millihertz, scale 1; mode is
borderless fullscreen and the backend reaches Immediate. This verifies
the corrected native geometry setup, not the performance target.

The probe ended unfocused; the full series needs you to focus each new
engine window before the measurement window begins at 22 seconds. Host
load was 2.74 before and 2.68 after, AC attached. The two attempts to reuse
the same output directory were refused before starting another engine,
as intended to preserve artifacts. The first successful result remains.

Next: full four-fixture series using the same `bin-08d`, fresh output
`/tmp/claude-gpu-pixel-cost/presented-external-08d`, and the previous full
series options with `--borderless`, without `--monitor-probe`. Agent idle
until you report done. The target remains pending.

## External timing series reviewed (2026-11-03)

`presented-external-08d` completed all 24 processes with `bin-08d` (SHA-256
`941df4b2c836cdcf...`), at SMAA and 100% scale, holes on, waits and GPU
markers off. All twelve logging-on windows have complete fixture windows,
readiness before flight, simulation advancement, fixture readiness, zero
rule-hole and fill-hole frames. No `no ground` occurs in any native log.

Every logging-on run reports external `Monitor #10002`, 144,000 millihertz,
2560 x 1440 physical surface, scale 1, borderless fullscreen, and Immediate
backend presentation after startup. The geometry fix held through the
whole series. Before/after load across all 24 runs ranges 2.65 to 10.87,
AC attached; thermal queries report no recorded warning/status.

Per-run frame distributions in ms, whole fixture window, without pooling:

| fixture | round | p50 | p95 | p99 | max | focused / frames |
| ------- | ----- | --- | --- | --- | --- | ---------------- |
| stationary | 1 | 4.41 | 6.05 | 8.00 | 11.01 | 0 / 2222 |
| stationary | 2 | 2.66 | 3.98 | 4.48 | 9.58 | 0 / 3501 |
| stationary | 3 | 2.62 | 3.89 | 4.48 | 5.88 | 0 / 3555 |
| flight | 1 | 4.50 | 6.40 | 9.10 | 17.53 | 212 / 2172 |
| flight | 2 | 2.97 | 4.53 | 5.19 | 9.41 | 0 / 3176 |
| flight | 3 | 2.77 | 4.27 | 5.00 | 9.45 | 0 / 3355 |
| sprint-flight | 1 | 4.36 | 6.07 | 7.71 | 13.44 | 0 / 2241 |
| sprint-flight | 2 | 2.70 | 4.63 | 5.78 | 12.87 | 0 / 3289 |
| sprint-flight | 3 | 2.77 | 4.55 | 5.40 | 9.72 | 0 / 3280 |
| sprint-walk | 1 | 2.98 | 4.55 | 5.51 | 13.63 | 0 / 3150 |
| sprint-walk | 2 | 2.83 | 4.27 | 4.96 | 10.22 | 0 / 3314 |
| sprint-walk | 3 | 2.85 | 4.43 | 5.25 | 7.65 | 0 / 3259 |

All twelve p95 values satisfy the numerical <= 6.9 ms threshold. This is
not yet an accepted focused-play target pass: eleven measured windows are
unfocused throughout, and flight round 1 has only 212 focused frames of
2,172. No run is fully focused. The focused subset is not substituted for
the whole flight window; all samples remain retained. Whether this is
actual app inactivity or a focus-reporting problem needs confirmation.

The first three runs have p50 about 4.4 to 4.5 ms, later runs about 2.6 to
3.0 ms despite the same final geometry and mostly identical unfocused
status. The earlier internal-display 8.4 ms state did not recur here, but
monitor, window mode, focus and host conditions differ, so this is not a
controlled causal explanation or a pooled comparison to the old baseline.
Logging-on/off pairs also vary substantially; their ratios cannot provide
a tight overhead bound for accepting a small optimization. No such claim
is made from this series.

The numerical target and geometry are recorded, the hole target remains
met, and 08 stays open for focused-play validation. A focused repeat or
verification/correction of the focus observation is needed before marking
the presented target met. No further presented run has been requested
while that observation is being clarified with you.

## Focus confirmed (2026-11-03)

You confirmed that another app was active during `presented-external-08d`.
Its focus telemetry is consistent with that observation; this is retained
as an unfocused series, not an accepted focused-play target pass. No
focus-reporting fix or rebuild is justified by that result.

Repeat with the same validated `bin-08d`, fresh output
`/tmp/claude-gpu-pixel-cost/presented-focused-08e`, and the same full
four-fixture options. Click each new engine window as it opens and keep
it the active app throughout the series. Do not switch to another app
between runs; each process creates a new window. The measured phase begins
22 seconds after launch. Expected duration remains about fourteen minutes.
The agent stays idle until you say done. Target acceptance remains open.

## Focused external target check (2026-11-03): target met

`presented-focused-08f` completed all 24 processes with the same validated
`bin-08d` (SHA-256 `941df4b2c836cdcf...`). SMAA, 100% render scale,
external `Monitor #10002` reporting 144,000 millihertz, 2560 x 1440,
scale 1, borderless fullscreen. Backend logs end in Immediate at this size
in all 24 runs. GPU and wait markers are off; holes are on.

All twelve measured windows are fully focused, complete, and ready before
flight, with simulation advancement and fixture readiness. Every measured
frame has zero rule-hole and fill-hole counts. No `no ground` appears in
any native log. Host load before/after ranges 4.79 to 7.38, AC attached;
thermal queries have no recorded warning/status. No samples were dropped.

Per-run frame distributions in ms, whole 22 to 32 s window:

| fixture | round | p50 | p95 | p99 | max |
| ------- | ----- | --- | --- | --- | --- |
| stationary | 1 | 4.47 | 6.45 | 8.18 | 11.87 |
| stationary | 2 | 4.29 | 5.64 | 6.41 | 9.16 |
| stationary | 3 | 4.30 | 5.66 | 6.23 | 12.21 |
| flight | 1 | 4.42 | 6.03 | 6.91 | 11.99 |
| flight | 2 | 4.50 | 6.02 | 6.71 | 17.22 |
| flight | 3 | 4.48 | 5.91 | 6.64 | 12.03 |
| sprint-flight | 1 | 4.60 | 6.26 | 8.28 | 14.58 |
| sprint-flight | 2 | 4.35 | 6.10 | 7.69 | 11.13 |
| sprint-flight | 3 | 4.37 | 5.99 | 7.74 | 14.04 |
| sprint-walk | 1 | 4.50 | 5.88 | 6.33 | 10.06 |
| sprint-walk | 2 | 4.46 | 5.83 | 6.28 | 9.71 |
| sprint-walk | 3 | 4.35 | 5.93 | 6.50 | 11.58 |

All twelve p95 measurements meet <= 6.9 ms; the worst is stationary round
1 at 6.454 ms. This establishes the focused rendered-frame target in the
verified external borderless condition. It is not a display-scanout
latency measurement, nor a promise that every frame is under 6.9 ms: p99
reaches 8.28 ms and the largest frame is 17.22 ms. Those tails are retained.
The spec's threshold is p95, not maximum.

Logging-on/off pairs still show variation above the small-effect overhead
bound in some cases (including stationary round 1). This series checks an
absolute threshold with logging enabled; it does not establish a small
optimization effect or a precise logging overhead estimate. No new
performance optimization is accepted on its basis.

The earlier unfocused external series has lower medians in its later
rounds than this focused series. The focus conditions are now confirmed
by you and the telemetry; those observations remain separate. Earlier
internal-display slow frames were dominated by window preparation, but
the exact driver/compositor wait and a controlled causal explanation of
every timing-state transition remain unresolved. The attribution checkbox
stays open rather than claiming that mechanism was proven. No additional
presented target repeat is needed: the current target has passed.

With the matched 1080p check and the hole checks, 07's performance targets
are now met for the recorded external borderless presentation condition.
Normal window and present-mode defaults remain unchanged. The remaining
08 work is causal attribution research, not a failed performance gate.

## Validation and authorization

frame-profiling.md. All heavy work goes through the shared heavy.sh lock.
Captures and raw artifacts stay under /tmp/claude-gpu-pixel-cost. Presented
runs need you at the Mac. Local work only; no push or PR without approval.
