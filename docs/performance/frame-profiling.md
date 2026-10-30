# Reproducible frame profiling

The native runner currently targets macOS. Build once, then run sequentially.
The `profiling` feature shares capture and statistics plumbing with `dev`,
without shader/asset watchers or the diagnostics HUD. Normal builds have
neither. No quality reduction or optimization is applied. The runner disables
the dev gallery in both profiles and adds identical fixtures through bounded
Server edits using declared Game content. No Client reads storage.

```sh
export CARGO_TARGET_DIR=/Users/mk/.cache/cargo-target
caffeinate -i env RUSTFLAGS="-D warnings" cargo build --locked \
  -p tesserae --features profiling --release
# Point --game at an already built demo distribution. No WIT/Game changes here.
# Use a fresh path: the runner refuses an existing output directory.
scripts/profile-frames --binary "$CARGO_TARGET_DIR/release/tesserae" \
  --profile release --features profiling --game /path/to/games/demo/dist/mods \
  /tmp/tesserae-release-unique
```

For matched dev evidence, build with `--features dev,profiling` without
`--release` and pass `--profile dev --features dev,profiling`. Record both
binaries before comparison; builds never overlap captures. The default series
has three interleaved rounds of all four fixtures, each with logging on and off.
A global runner lock and process checks reject competing Cargo, engine and known
bench processes. This cannot reserve the host against unrelated external
activity; inspect logs and reject contended rounds rather than deleting tail
samples.

`--view third` measures the over-the-shoulder camera; the default is
first-person, and an engine without views ignores the option, so one runner
serves an A/B against an older build.

Use `--presented` for native presentation; default is 1920 x 1080 offscreen and
works with a locked display. This is rendered throughput, not headless
throughput or display latency. Requested `AutoNoVsync` is recorded;
backend-resolved fallback and actual present elapsed time are unavailable.
Presented window dimensions are recorded at runtime. Do not pool presented and
offscreen measurements.

Each run retains its PNG, log, environment, executable hash and raw samples. The
series retains a copied executable, Engine source hashes, source revision/diff,
Game file hashes, build profile/features and host identity. Keep native
artifacts in the unique `/tmp` review folder; never commit them. Record
AC/thermal state, physical display and other host activity alongside accepted
comparisons.

## Clocks and phases

`frames.csv` IDs identify a First-to-next-First interval with its own main
schedule span and costs. Both use monotonic `Instant`; no virtual-time delta is
used for measurement. Startup before the first First is separately retained in
metadata; startup stalls are never clamped to 250 ms. The final incomplete
interval is excluded. Scheduling phases use real time; intentional movement
still uses Bevy's virtual delta and the existing Client pose report/Server
validation path.

Boundaries are 0–16 s startup, 16–20 s stationary, 20–22 s transition, 22–32 s
fixture, then settle. Flight starts at 20 s after readiness (or the existing 30
s fill timeout); reject flight results when streaming was not settled by 20 s.
Stationary, transparency and active fixtures have zero flight speed. Transparent
geometry is six 12 x 6 blended planes ahead of spawn. Active geometry is a fluid
curtain with a separate burning declared fuel cell. Construction is 24 edits per
tick, through the existing diagnostic builder. Two distinct fuel cells ignite at
20 and 26 seconds, preserving existing burn timers while keeping fire active
throughout the measurement window. The runner rejects a logging-on active run if
any measured frame lacks burning fuel. Logs mark completion; reject a run whose
fixture was not constructed before its measurement window. Seeds and geometry
are fixed; asynchronous readiness and simulation progress are observations, not
promised identical frame counts. These fixtures explicitly use free flight and
first person; later controllers must keep this diagnostic path or introduce a
separately identified equivalent fixture.

`streaming_shown` means the Server delivered initial mesh coverage; it does not
prove GPU pipeline readiness. Native captures verify loaded visible output.
Streaming shown/settled and Server simulation step, idle, running, awaiting,
suspended, timers, cumulative landed and deferred counters are distinct columns.
Idle means no running step or Agenda work; timers/awaiting may still exist. A
stationary view does not imply idle simulation. Fixture readiness, ignition
counts and actual burning state are also exported. Active windows must show
actual simulation progress/work, rather than merely a scene named “active”.

Main wall is First to Last; it includes waits and excludes extraction/render.
Main CPU uses the current thread clock on Unix, empty when unsupported. Existing
Server tick, mesh construction/upload and blended sort hooks measure wall time;
upload does not claim GPU transfer time. Catch-up tick totals and longest tick
are separate. `render.csv` records the render schedule from before
ExtractCommands to after PostCleanup with the extracted main frame ID, including
scheduling waits. It records that thread's CPU independently. Render and main
may overlap. Never add their percentiles, per-view spans or nested spans to
obtain frame elapsed. Extraction and presentation elapsed remain explicitly
unavailable; the failed historical Metal trace supplies no timing evidence. GPU
elapsed comes from `frames.gpu.csv`; see Streaming fixtures below. Render export
may omit the final in-flight render frame at shutdown.

The profiler buffers at most 250,000 frames plus 250,000 render rows, exports
once at shutdown or the limit, and marks limit exhaustion. There is no per-frame
disk logging. Existing dev statistics still run independently of HUD visibility.
`TESSERAE_PROFILE_CSV` alone opts into sampling; continuous update is separately
matched by the runner for on/off comparisons. Reused raw output names fail.

## Evidence gates

`summary.json` derives nearest-rank p50/p95/p99/max per phase from retained raw
rows. Reanalyze with `scripts/profile-frames --analyze-only /tmp/run-folder`.
Compare per-run distributions across at least three interleaved rounds; retain
startup/transition tails separately. Logging-off legacy capture summaries use
real time but are rounded: use their ranges to bound overhead, never claim
sub-rounding improvements. If overhead exceeds 3% of median or 6% of p99, or the
run ranges overlap that bound, collect a longer independent reference before
using the profiler to accept an optimization.

Select later optimization gates only from uncontended matched release evidence.
Require an effect exceeding both the logging overhead bound and between-run
variation, with no regression in p99, streaming readiness or simulation
progress. The historical 6.99 ms dev/offscreen median and 12.66 ms p99 are
context, not a release gate; 1,000+ FPS remains a stretch investigation. Quality
probes cannot count as equal-quality improvements. GPU/per-view optimizations
are attributed by `frames.gpu.csv` within the limits below; static atlas/UI
changes require equivalent output evidence.

Core algorithm comparisons retain phase-8.2 ticket 13's 115-row methodology:
separate temporary targets and copied executable hashes, foreground QoS,
thread-CPU time, at least three interleaved rounds and equal-output assertions.
Exclude aliased targets and contended runs. Preserve its documented
binary-layout, id-move and allocation/hash-state exceptions; percentages on
nanosecond rows do not establish frame savings. These rendered fixtures do not
replace that A/B.

The
[P0 validation record](../../.scratch/2026-10-27-gameplay-performance-and-controls/implementation-validation.md)
retains the initial release evidence and exclusions. Its stable follow-up flight
reference has observed conservative logging bounds of 2% median and 6% p99 after
summary rounding. Use a 5% median improvement gate, beyond matched run
variation, with non-overlapping run ranges, no consistent p99 regression and
preserved streaming/simulation progress. Rebaseline other fixtures before
accepting small effects; the broad series was noisy.

For the focused reference, keep the same executable/profile/Game arguments and
pass `--fixtures flight -- /tmp/fresh-flight-reference` to the runner. The `--`
terminates the fixture list before the output path. Three interleaved rounds
remain mandatory.

## World save events

Persistent fixtures append `save_snapshots`, `save_snapshot_ms`,
`save_worker_ms` and `save_chunks` to the raw frame rows. An explicit disposable
`TESSERAE_WORLD_DIR` enables saves for a capture; fixtures otherwise remain
ephemeral. `TESSERAE_EPHEMERAL=1` always disables persistence.

The snapshot counter increments when the Server dispatches an immutable save.
Snapshot milliseconds measure its capture work on the Server, already included
in the full tick/frame spans. Worker milliseconds describe the latest completed
encoding/validation/checksum/write task, including filesystem waits; they are
wall time, not worker CPU. Values persist across rows until the next event, so
select counter changes to identify snapshots rather than treating each row as a
new save. A worker result can appear on a later frame than its snapshot; check
the retained commit files and native diagnostics for successful writes. These
fields do not establish power-loss safety, GPU cost or present latency.

## Streaming fixtures

`--fixtures` also accepts four streaming fixtures, all on the demo's default
seed and measured in the same 22–32 s phase:

- `sprint-flight`: noclip at 64 m/s, the flight sprint cap, along the default
  heading.
- `sprint-walk`: physics sprint at 8 m/s along the default heading, which stays
  on walkable ground for the whole run.
- `dive`: noclip straight up 320 m over 22–27 s and straight back down over
  27–32 s, both at 64 m/s, so both directions of a fast vertical move are
  measured from the same ground.
- `teleport`: the Server moves the Player 2 km along x at 22 s, keeping its
  height above the generated ground; `streaming_settled` gives the time to
  refill.

Each run adds three files beside `frames.csv`, and the runner summarises them in
`streaming.json`:

- `frames.csv` gains `rule_holes` and `fill_holes` with per-level columns,
  `chunk_entities` and `visible_meshes_camera<order>`. A hole is a Cell of a
  level's mesh shell that intersects the Player camera's frustum, out to the LOD
  reach, and that nothing draws: a _rule hole_ where some level delivered the
  region but the draw rule in force discards it, a _fill hole_ where no level
  delivered it. It is counted on the Client from what it was sent, never from
  World storage. The rule in force is the metre cut; a Cell the cut's edge
  crosses is split up to three levels finer, then tested at its centre. A hole
  hidden behind nearer terrain still counts.
- `frames.flow.csv` has one row per level per fixed tick: Chunks held and
  meshed, and per stage (generate, relight, mesh) the Chunks waiting to
  dispatch, in flight, and since the World opened the jobs run and their worker
  thread CPU, plus fluid jobs run and their worker CPU. `streaming.json` reports
  worker milliseconds per second over the measured phase.
- `frames.gpu.csv` has GPU timestamps per frame: `frame_start`, `frame_end`, and
  per camera order `view_start`, `prepass_end`, `main_start`, `opaque_end`,
  `main_end` and `view_end`, in milliseconds since `frame_start`. Apple GPUs
  sample timestamps only at encoder boundaries, so each marker is an empty
  compute pass dispatching one workgroup between two of Bevy's passes. A marker
  is ordered by the schedule: work the GPU overlaps across one is charged to
  whichever side it ends on, systems outside Bevy's sets can fall on either side
  of `view_start`, and `prepass_end` to `main_start` includes the shadow passes.
  About 10% of frames return an earlier frame's value for the last markers;
  those cells are left empty, and `streaming.json` reports how many frames have
  a `frame_end`.
