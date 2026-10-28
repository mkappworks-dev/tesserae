# Performance evidence and proposed PRs

Status: research. Measurements apply to the stated fixture and machine only.

## What 100 and 1000 FPS mean

100 FPS is 10 ms per rendered frame; 1000 FPS is 1 ms. The latter needs a
tenfold reduction of the critical frame path if 10 ms is truly work, rather
than waiting on a presentation cap. A fast core kernel or headless tick is
not a rendered frame. CPU and GPU run in parallel; adding their elapsed
times, or subtracting unrelated percentiles, does not recover frame time.

- Display refresh: the number of images a display can actually show.
- Render throughput: completed rendered frames over time, including any
  presentation/backpressure. An FPS counter alone does not identify that wait.
- Main schedule wall time: First to Last in the main Bevy schedule, including
  Server ticks and UI but excluding subsequent render extraction/sub-app work.
  This is wall time, not thread CPU or a complete CPU frame budget.
- Server tick: fixed at 64 Hz, not raised to 1000 Hz by a high render rate.
  A busy simulation can take several ticks per Logical step under ADR-0014.
- Headless throughput: World generation, meshing, collision and simulation
  benches; these omit actual gameplay presentation and are not FPS claims.

Normal `main.rs` leaves the Window present mode at Bevy 0.20's default
`Fifo` (vsync). The capture path switches to `AutoNoVsync`, which is a
fallback selection, not proof that every backend presents without waiting.
There is no explicit application frame cap in current Tesserae code.
Bevy's default unfocused window update policy also uses a 60 Hz timer;
temporary profiling explicitly uses continuous updates to remove that timer.

## Machine and controlled fixture

- Apple M5, 10 CPU cores (4 performance/Super, 6 efficiency), 10 GPU cores,
  24 GB memory, macOS 27.0.1, native Metal adapter.
- AC connected, battery at 80%. `pmset` reported no recorded thermal or
  performance warning; this is not a temperature, frequency or power trace.
- Built-in Retina panel: 3024 x 1964 physical pixels. External panel:
  2560 x 1440 at 144 Hz. The created window reported 2560 x 1440 physical,
  scale 2, unfocused. Rendering below targets a separate 1920 x 1080 image.
- Rust 1.97.1, `RUSTFLAGS="-D warnings"`, dev profile: workspace opt-level 1,
  external dependencies opt-level 3. Features: `tesserae/dev`. This is not
  a release build, even though dependencies are optimized.
- Source `7907b6d` plus temporary capture instrumentation. No content/WIT or
  simulation rule changes. The demo and fixture guests were built through
  `scripts/build-guests` using the existing shared cache.
- Default seed `0x7e55_e2ae_0000_0001`, 1 m Voxels, demo Game, declared
  full-detail distance 160 m and LOD reach 3000 m. Caves, smoothing, textures,
  fog, sun shadows and four-sample MSAA retain their normal settings.
  No added SSAO/TAA/bloom is assumed from Bevy feature availability.
- Six World LOD cameras, one corner-held-model camera and one item-atlas
  camera: eight active cameras in baseline logs. Atlas itself has MSAA off.
- Native offscreen capture, continuous updates, `AutoNoVsync`; 20 seconds
  before flight, 12 seconds flying forward at 12 m/s and default pitch,
  1 second final settle before screenshot. No user input is synthesized.
- Analysis uses `(16,20]` seconds for the preflight window and `(22,32]`
  for the flight window, omitting flight transition. Each run's raw CSV and
  phase boundaries are retained. Statistics use nearest-rank percentiles.
- One process at a time, `caffeinate -i`, no simultaneous Cargo/bench/capture
  process found before the series. Variant order is interleaved/reversed.
  A shared host can still have unrelated activity; tail spikes are retained.

The user confirmed the baseline question with "yes" but did not identify
display, window size or activity. Using `cargo dev` is the working assumption;
the original 100 FPS report has not been reproduced at known display settings.

Offscreen removes the displayed presentation path. These results describe
this rendered fixture, not display latency or a guarantee for normal play.
The final captures were inspected to confirm actual textured terrain and HUD,
not black or unloaded frames. The menu case renders the World behind its UI.

## Existing benchmark evidence remains separate

Read phase-8.2 ticket 13 before proposing any core optimization. Its comparison
uses actual phase-8.1 `origin/main` at `212d9008`, not the older local main.
Four clean interleaved thread-CPU rounds cover 115 rows, with longer targeted
and same-binary id probes. Shared-target aliasing and contended rounds were
excluded before accepting evidence. The results do not establish a changed
mesher or coarse derivation, and the large stair shifts were isolated as
binary-layout effects. Sub-microsecond allocation/hash-state outliers are
not meaningful frame savings. No new core A/B is claimed here.

Any future algorithm change must preserve that method: unique temporary
workspaces and targets, executable hashes, foreground QoS, thread CPU,
interleaved order, equal fixtures/output assertions and no competing run.
Do not reuse or overwrite the previous session's raw artifacts.

## P0: an actionable frame profiler

Problem: the FPS overlay is a smoothed frame counter, current capture logs
round milliseconds to one decimal, and current CPU hooks cover Server tick,
mesh construction/upload and blended sorting only. Hidden diagnostics still
format/update text, and dev statistics publish separately from visible HUD.

Scope: opt-in buffered CSV/trace export with metadata, exact phase markers,
raw real frame deltas, main-thread CPU and wall time, render-thread CPU,
mesh bytes/uploads, per-view render work and GPU/present timings when the
backend supports them. Include actual selected present mode, focus, physical
target dimensions, render scale, content hashes, build profile, view/LOD and
simulation backlog. Separate draw readiness from simulation/Mod completion.
Provide deterministic stationary, moving, transparent scene and active
fluid/fire fixtures; keep instrumentation out of normal play when disabled.

Acceptance: a checked-in reproduction command emits p50/p95/p99/max for each
phase over at least three interleaved runs; native presented and offscreen
results are clearly separated; unsupported GPU queries say unavailable.
Compare logging-on/off to bound measurement overhead. Preserve captures and
thread-CPU methodology. Do not interpret the sum of nested GPU spans as the
whole frame, or label a render scheduling wait as CPU computation.

## P1: render static preview images only when dirty

Problem: `tile_preview::update` caches geometry and the image, but leaves its
Camera active every frame. The item set and textures are usually unchanged.

Scope: render once after a new atlas/texture/orientation set becomes ready,
then reuse the image. Reactivate on content changes and keep rendering until
mesh, material and pipeline readiness is confirmed; a one-frame blind timeout
would cache black or incomplete thumbnails. Preserve all approved models,
alpha blending, orientation, drag feedback and atlas asset lifetimes.

Acceptance: identical current-quality item screenshots after startup, menu
open, drag/drop, slot swap, content hot reload and Shape rotation; no blank
icons after shader compilation; no asset leak. A/B flight and menu cases
with profiler pass counts confirm the atlas pass stops when clean. Report
the actual frame delta; freezing after five seconds is only a diagnostic
upper-bound probe and does not implement this lifecycle.

## P2: quiet UI should stop changing

Problem: hidden `overlay::update_readout` formats FPS and writes text; removal
bar/place-hint paths write Nodes/Visibility even while inactive. Atlas input
collection also allocates/sorts a wanted list each frame. These are code-level
work candidates, not yet attributed millisecond bottlenecks.

Scope: run conditions and `set_if_neq`/change-driven updates for inactive
panels and stable preview inputs. Throttle diagnostics only as appropriate
for readable text; do not throttle gameplay target feedback or hold progress.
Consider dev statistics collection separately from display so profiling keeps
working when the HUD is hidden. Retain a clear debugging path.

Acceptance: unchanged appearance and fresh feedback on first show, correct
hold progress, no UI rebuild/layout invalidation during a quiet trace, and
no missed catalogue/texture updates. Measure system/layout time and a
current-quality A/B before claiming an FPS gain. Reject added caches if the
complexity outweighs measured savings.

## P3: reduce per-view rendering overhead at equal appearance

Problem: the LOD priority scheme uses several full-target cameras; the held
model has another. Resolution, shadow and MSAA probes identify sensitivity,
but removing quality is not an optimization at comparable quality.

Scope only after P0's native GPU/CPU capture: inspect redundant passes,
MSAA writeback/resolve, tonemapping, visibility/queue work and shadow view
work per camera. Test minimal camera settings for the held model and atlas,
then one narrow measured change per PR. Preserve finer-level coverage
priority, ring joins, transparent coverage, texture alpha and screenshots.
Do not consolidate all LOD cameras into shared depth without a new proof:
the existing design records why a coarse depth push failed at grazing views.

Acceptance: same output/quality at default and demo reach, low grazing angle,
lakes, glass, cutouts, 1/0.5/0.25 m; per-view CPU/GPU improvement and no worse
streaming p99/fill time. Quality settings such as lower resolution, shadows
or antialiasing form a separate explicitly labeled user settings PR, not
an invisible change to defaults. Renderer rewrites are unsupported here.

## P4: safely batch emitter relight

Historical evidence: phase-8 simulation measurements found the costliest
0.25 m lava drain step at 11.2 ms of main-thread CPU in dev. Four emitters
per Logical step was deliberately chosen; raising it to eight reached
13.75 ms. That is a simulation tail, not a universal normal-frame cost.

Scope: profile repeated incremental light work and design a bounded bulk
relight/landing path, keeping exact Occupants, Lifetime, wake, light-settle,
Edit and LOD invalidation rules. Keep the existing bounds and Mod drain cap
unless a separate same-outcome measurement justifies a reviewed change.

Acceptance: deterministic equal ordered-event results at all three Voxel
sizes and worker counts; no delayed/lost player Edit; compare thread-CPU
tick distribution, settled visibility latency and simulation backlog, not
just uncapped render FPS. This is independent of remapping and inventory.

## Evidence location and checks

Raw evidence: `/tmp/tesserae-gameplay-research-gRUJEoDZ`.
`run.py`, `analyze.py`, the source backup, instrumentation patch, per-run
CSV/log/metadata, screenshots and `summary.json` retain the reproduction.
The diagnostics pilot is reported separately because instrumentation differs.
Initial launch with the wrong unbuilt Game path exited before gameplay and
is not a performance result.

Reproduction is in [reproduce.md](reproduce.md), with an archived research
patch and portable driver/analyzer. Compact aggregate data is retained in
[measurement-summary.csv](measurement-summary.csv). All raw native artifacts
remain outside git.

## Measured results

Each row reports the median across runs of each run's frame-time percentile;
the range is the minimum/maximum run p50, not a confidence interval. All
figures are milliseconds. The accepted window is ten seconds per run.

| Case | Runs | p50 | p95 | p99 | Run p50 range |
| --- | --- | --- | --- | --- | --- |
| Current quality, flight | 4 | 6.99 | 10.10 | 12.66 | 6.48–7.38 |
| Stationary camera | 3 | 6.53 | 8.79 | 10.17 | 6.51–6.68 |
| Frozen icon atlas | 3 | 6.42 | 9.44 | 13.82 | 6.33–7.11 |
| 960 x 540 only | 3 | 6.30 | 10.17 | 14.86 | 6.22–7.17 |
| Catalogue open | 3 | 7.08 | 10.28 | 14.95 | 6.89–7.08 |
| Sun shadows off | 3 | 6.03 | 8.57 | 10.70 | 5.85–6.68 |
| MSAA off | 3 | 7.21 | 10.66 | 14.81 | 6.33–7.22 |
| Combined quality reduction | 2 | 2.45 | 3.81 | 4.78 | 2.44–2.45 |

Current-quality median throughput is about 143 FPS by reciprocal median frame
time. It is not 143 consistently displayed frames per second. The four run
p99s range from 10.51 to 13.42 ms; the worst frame in the accepted windows
was 37.43 ms. Do not label the reciprocal of a p99 an averaged "1% low FPS."

The combined reduction is 960 x 540, LOD disabled beyond the demo's 160 m
full-detail reach, no sun shadows, no MSAA and atlas rendering frozen after
warmup. It drops from eight to four cameras. About 408 FPS is demonstrable
in that deliberately different scene, still well short of 1000. The two
runs are exploratory characterization, not a measured optimization gain.
Capture inspection confirms reduced distant coverage and cramped HUD at this
resolution; neither is a replacement for the approved design.

Baseline drawing is complete early: first level-0 display at 1.04–1.35 s,
level-0 settled at 1.64–1.94 s; the other levels fill before the measurement
window. End-of-flight snapshots have 1892 held level-0 Chunks, 672 meshed
(including empty) and 557 drawn terrain mesh entities. Mean mesh upload
construction cost over each complete run is 13–14 us/mesh. These counts
describe this terrain/heading, not every World.

One-minute host load ranged roughly 4.8–8.6 across the normal series. No
concurrent Tesserae benchmark/build was launched by this session. The shared
host remains a source of noise; run medians drift by about 0.9 ms even in
baseline, and outlier tails remain in the data. Native sample/Metal captures
are separate cases and never included in these rows.

## What the evidence does and does not identify

**CPU main schedule:** median 2.06 ms in flight across baseline runs, versus
1.68 ms with a stationary camera. This already exceeds a whole 1 ms frame
budget, while excluding render extraction. It is wall time, including waits;
it does not establish a 2 ms CPU instruction workload. The combined reduced
scene still has about 1.1 ms median main-schedule wall time.

A separate five-second native `sample` capture at approximately 22 seconds
shows main-thread waits through Bevy's task executor and pipelined renderer
extraction, render-thread Metal command/driver work, and lighting flood work
on workers. Its 902 sampling rounds and very large frame perturbation make
it qualitative stack evidence only. All-thread collapsed sample counts
include sleeping workers and cannot be interpreted as CPU utilization or
percent of frame cost. In particular, a main-thread wait is not evidence
that the thread itself needs an algorithm rewrite.

**Server, streaming and transparent sort:** baseline run p99s for their
combined existing hooks are 2.57–3.14 ms in the accepted flight window.
The stationary runs are 1.03–1.11 ms. Thus movement/streaming contributes
work, but those hooks alone cannot explain the whole rendered frame.
Server ticks include simulation and Mod work; this capture does not isolate
each from streaming. Individual catch-up ticks and sorting can still spike.
Whole-run worst sort intervals are 1.43–1.97 ms despite a nominal 200 us
sort budget: the budget can overrun by the last sort and wall time includes
descheduling. These are not claims that every frame sorts for that long.

**GPU/rendering:** lower resolution and shadow removal suggest sensitivity
to rendering work. Resolution changes pixels by fourfold, yet does not come
close to a fourfold frame reduction. Multiple cameras, per-view CPU work,
GPU work and scheduling can all contribute. These probes do not isolate
shader throughput, GPU occupancy, MSAA resolves or presentation wait.
Shadow removal helped two rounds and overlapped baseline variation in the
third. MSAA alone gave no repeatable aggregate improvement. Neither supports
a promised speedup or silently disabling quality.

The optional Bevy render diagnostics emitted CPU pass values but no GPU
elapsed values on this setup. A separate Instruments Metal System Trace
reached its five-second recording limit but did not finalize within the
45-second profiler deadline. The process was stopped by the timeout; no
Tesserae or recording process remained. Export subsequently failed with
`Document Missing Template Error`. The incomplete native trace is retained
in the temporary folder, and supplies no usable GPU timing. GPU pass cost,
queue wait and overdraw therefore remain explicit measurement gaps for P0;
the resolution probes do not fill those gaps.

**Transparent overdraw:** current CPU sorting is timed separately. Actual
transparent GPU overdraw is not identified by that CPU time. A default
grassland flight is not a dense glass/water stress scene, and no result here
justifies replacing correct transparent ordering with a cheaper wrong order.
P0 needs matched high/low-overdraw fixtures and native GPU pass evidence.

**UI:** atlas freezing shows a promising median shift but its ranges overlap
baseline and its p99 is worse. The menu case is broadly comparable to normal
play, not evidence that the catalogue dominates the frame. P1 is a bounded
candidate with an obvious unnecessary recurring render; P2 remains a code
inspection hypothesis until system/layout timings and a targeted A/B verify
it. Keep these small rather than claiming that UI explains 100 FPS.

**Simulation:** the default 1 m Game is loaded and can have ongoing fluid,
fire and Mod construction in the background. A stationary camera is not
proof of idle simulation. This investigation read the actual existing
thread-CPU measurement path and its phase-8 evidence; it did not rerun a
0.25 m lava/fire stress sweep or turn those figures into FPS. The existing
64 Hz fixture measures tick CPU/wall, backlog and source-to-visible latency
separately. Its 11.2 ms historical worst CPU tick is precisely why a 1 ms
render target also needs a plan for occasional Server landing work.

## Recommended targets and limits

These were research recommendations, not agreed acceptance requirements.
During grilling the user agreed to preserve current visuals/view distance
for normal-play improvements, keep reduced-quality maximum-FPS modes optional,
and retain 1000+ FPS as a stretch investigation. The 144 Hz proposal below
is not a ceiling. Uncapped release measurements are still required.

The user also selected display-synced normal play by default, configurable
FPS caps and an explicit uncapped benchmarking option. A capped FPS counter
must not be treated as renderer capacity. These settings change presentation
policy, not visual quality, view distance or the Server's simulation rate.

1. At comparable quality, first target a better **144 Hz experience** on the
   reference M5: p95 at or below 6.94 ms and p99 at or below 10 ms for the
   defined flight, with no worse streaming readiness. These are proposed
   acceptance goals, not results already achieved or a guarantee for all
   Worlds. Verify native presentation on the actual display before claiming
   it. The current p50 is near this budget; its tails need work.
2. Treat **200–240 FPS** at comparable quality as a later stretch experiment
   requiring an actual release build, per-view CPU/GPU evidence and matched
   content. No 4.17–5 ms comparable-quality result is established here.
3. A **1 ms mode** must budget the entire CPU/render/GPU critical path below
   1 ms, including a strategy for fixed-tick spikes. The measured combined
   quality reduction still takes 2.45 ms. Smaller targets, fewer views,
   simpler shading and reduced reach are plausible experiment parameters,
   not a sufficient recipe or approval to change defaults. Replacing the
   renderer or bypassing Server authority is unsupported by this evidence.

Release throughput remains unmeasured. The existing cached release Bevy has
different features from the dev capture build. This session avoided a new
large Bevy feature variant solely to advertise a release FPS figure. P0
should make matched release capture available without requiring hot-reload
watchers, with explicit feature/profile metadata. Do not infer release
speed from debug/optimized-dependency settings or old core benchmarks.

Prioritize P0, then a focused P1/P2 validation, then P3 from actual per-view
evidence. P4 addresses simulation tails under its own deterministic fixture.
There is no measured basis for a speculative mesher/engine rewrite, larger
simulation bounds, or a promise of 1000+ FPS at current quality.

## Final research verification

- Temporary profiling build and restored normal build use `caffeinate -i`,
  warnings denied, `--workspace --features tesserae/dev --locked`.
- Guest/demo build script completed. No WIT or Game source changed.
- Accepted windows were checked for complete duration and absence of
  virtual-time clamping; main-span/frame row counts match.
- Profiling scripts parse; recorded aggregates match the raw CSVs.
- Product source was restored byte-for-byte to HEAD. The normal executable
  is rebuilt into the original shared cache; no new Bevy variant is needed.
- Research Markdown is wrapped by hand at 80 columns; no Prettier ran on
  `.scratch`. Full CI/tests were not rerun for research-only notes.
- GitHub's successful head checks and the previous 902-test local validation
  remain the product evidence; they are not new test results from this work.
- No feature was implemented, no branch was pushed, and no PR was changed.
