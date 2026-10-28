# P0 implementation and validation

Tickets 11–12 only. At validation time, changes were local and uncommitted;
no push, PR publication or merge had occurred.
Source base remains 7907b6de57e337d6257fc5d345c27e098db0b6d1.
Other tickets remain planning artifacts. The source research workspace was
read/copied and never edited.

## Implementation

The dedicated `profiling` feature enables capture/statistics in release
without asset watchers, the dev gallery or the diagnostics HUD. The raw
profiler is opt-in through TESSERAE_PROFILE_CSV. Sampling is independent of
HUD visibility, buffered and capped at 250,000 main and render rows.

Main frame IDs join First-to-next-First intervals, First/Last wall and
thread CPU, Server tick/upload/sort wall costs, streaming and simulation
status. Render IDs are extracted from the associated main frame; exclusive
render boundary systems measure that thread's wall/CPU independently.
First/Last main hooks are pinned to the main thread. Startup before First
is retained separately. Virtual movement semantics remain unchanged.
Extraction, GPU and present elapsed times are explicitly unavailable;
requested present policy is recorded without inferring backend fallback.

The Server publishes simulation/fixture snapshots. Dense transparency and
fluid/fire geometry use declared Game content and the bounded diagnostic
builder/edit path. Fire ignites distinct cells at 20 and 26 seconds: setting
the same burning state preserves its timer, so a repeated write is not a
valid way to renew fire. No simulation rule or bound was changed.

The sequential runner copies/hashes the executable, records Engine/Game
provenance and exact profile/features/environment, checks known competing
processes, interleaves at least three rounds, and retains raw distributions
and native captures. It rejects incomplete/unready measurement windows and
any active logging-on frame without burning fixture fuel. Offscreen and
presented paths have separate labels. The runner currently targets macOS.

Reproduction and interpretation: docs/performance/frame-profiling.md.
Compact numerical data: release-summary.csv beside this document.

## Checks

- All cargo-ci checks passed, with its guest-build command replaced by
  reuse of existing guest/demo artifacts. WIT/Game sources did not change;
  the user explicitly restricted unnecessary guest rebuilding. Workspace
  tests: 902 passed, zero failures, five existing ignored measurements.
- Final profiling/fixture regressions: four passed. These cover unclamped
  750 ms intervals/frame identity, disabled sampling and fire scheduling.
- Python analyzer regressions: two passed, with warnings treated as errors.
- Final all-feature Clippy, release build, rustdoc, default-feature check,
  benchmark compilation, boundaries and dependency checks passed.
- No core algorithm, WIT, Game, phase-8.2 content model, controls or product
  presentation policy changed. Existing capture users remain supported.

Cargo used caffeinate, RUSTFLAGS="-D warnings" and the shared cache at
/Users/mk/.cache/cargo-target. Guests/demo were reused, not rebuilt.

## Native evidence

All native artifacts remain under:
/tmp/tesserae-p0-review-qbkdybeu

The final release binary SHA-256 is:
9735fb6baf80a2e7356ae1b23d5c37072d9e92e5a0d90efcd29eb0bfdd8860b2

`release-final` contains 24 successful captures: three interleaved rounds
of four fixtures, each logging on/off. Its 12 raw exports retain 56,891
main frames and 56,882 render rows. All windows completed and were ready;
every active measurement frame had burning fuel. Simulation advanced
639 steps in each active window and landed 180–181 changes.

The broad series has substantial host drift and is functional fixture
coverage/characterization, not accepted evidence of optimization gains or
small profiler overhead. Its logging-on/off median ratios range from
0.762 to 1.164 and p99 ratios from 0.362 to 2.411. Tails remain retained.

`flight-overhead` contains six additional captures: three interleaved
flight on/off pairs using the exact same executable. It retains another
15,048 main frames and 15,046 render rows. This series was substantially
steadier and is the provisional matched release flight reference:

| Metric | Median of three run percentiles, ms |
| --- | ---: |
| Frame p50 | 5.953500 |
| Frame p95 | 8.240042 |
| Frame p99 | 9.379250 |
| Per-run frame maximum | 14.327333 |
| Main thread CPU p50 | 0.496334 |
| Main schedule wall p50 | 1.580167 |
| Render thread CPU p50 | 2.807041 |
| Render schedule wall p50 | 5.611000 |

Run p50s are 5.854875, 5.953500 and 6.021459 ms. Do not add main/render
values or subtract unrelated percentiles. These are rendered application
frame intervals, not GPU elapsed time or display latency.

Legacy capture summaries cover the matched 12-second flight and round to
0.1 ms. Their on/off p50 pairs are 5.9/5.9, 5.9/6.0 and 6.0/6.0 ms;
p99 pairs are 9.4/9.2, 9.0/8.8 and 9.2/8.8 ms. Allowing +/-0.05 ms per
rounded value yields conservative observed upper differences below 2%
for p50 and 6% for p99. These are observational bounds including noise,
not a proof of intrinsic instrumentation cost on every host or fixture.

For later flight optimizations, require at least a 5% p50 effect, larger
than the 2.85% between-run p50 spread and the 2% median overhead bound,
with non-overlapping matched run ranges and no consistent p99 regression.
Treat p99 changes below 6% as unresolved by this reference; reproduce
smaller effects with longer independent measurements. Require streaming
readiness and simulation progress not to regress. Other fixtures need a
fresh stable matched baseline before accepting small numerical effects.
These are evidence gates, not a promise of 1000+ FPS or a product cap.

Hardware: M5 10-core CPU/GPU, 24 GB, Metal, AC attached, 1 m Voxels,
fixed seed 0x7e55e2ae00000001, demo Game, 160/3000 m near/LOD, normal
shadows/MSAA/textures, free flight/first person, continuous updates,
1920 x 1080 offscreen, unfocused window, requested AutoNoVsync. Actual
physical camera targets, focus, Engine/Game hashes and adapter logs are
retained. Eight cameras remain active; no atlas/UI optimization ran.

A host probe during the broad series reported load 12.25/11.00/10.58.
System simulator background tasks were also observed. No competing
Cargo or known Tesserae benchmark ran. Thermal/performance queries failed
with 0xe00002bc; no temperature/frequency or thermal-limit claim is made.
Shared-host activity is a material limitation on small effects.

`smoke` validates native loaded output and retained a 374 ms real startup
interval. The synthetic regression retains an unclamped 750 ms interval.
`release-preliminary-excluded` and `release-partial-fire-excluded` retain
abandoned fixture pilots; they are excluded from every final summary.
Their processes were stopped before any corrective build.

No native presented or GPU timing claim is made. The earlier historical
Metal trace is not timing evidence. No historical dev/release speedup is
claimed. Phase-8.2 ticket 13's 115-row thread-CPU A/B, separate targets,
foreground QoS, interleaving, equal-output checks and layout/id exceptions
remain the methodology for future core changes; no new core A/B ran.

## Publication authorization

The user subsequently authorized opening PR 1. Commit/push/publication
are authorized for this scope; merging and later feature work remain separate.
