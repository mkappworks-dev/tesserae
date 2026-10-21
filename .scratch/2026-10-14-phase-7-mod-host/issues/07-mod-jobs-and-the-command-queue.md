# Mod Jobs, the Command queue and its drain

Status: done
Blocked by: 05, 06

Spec decisions 7, 8 and 9 in the Server. The ticket where a Mod changes the
World.

## What to build

- `server/mods.rs`, a plugin: a `ModJobs` resource that owns each `ModInstance`
  while it is idle and the `Task` while it is running, in the pattern
  `server/jobs.rs` uses (`AsyncComputeTaskPool`, a map of running tasks, polled
  once per tick). One call per Mod at a time; the call is started in
  `FixedPostUpdate` and landed in the next tick that finds it finished.
- The Edit log: `edit::apply_pending` records each **player** Edit that applies
  as an `EditEvent` (position in metres, before and after) in a bounded buffer;
  a Mod's `TickContext` takes everything since its last call. Mod-origin and
  `dev`-builder Edits are not recorded. A full buffer drops the oldest and
  counts it.
- Landing: a finished task returns `Result<CommandBatch, ModFault>`. `Ok` pushes
  to `PendingCommands`; `Err` drops the instance, writes a `ModDisabled` message
  with the Mod's name and reason, and logs once.
- `PendingCommands` (`pub(in crate::server)`): per Mod, a FIFO of validated
  Commands with a fill cursor. `drain` in `FixedPreUpdate` right after
  `apply_pending`: up to `MOD_VOXELS_PER_TICK` Voxels, round-robin across Mods,
  each through `edit::set_voxel` exactly as `apply_pending` calls it. A Voxel
  whose Chunk is not held is skipped; one `set_voxel` rejects is skipped; both
  count into the Mod's next `BatchReport`.
- Backpressure: a Mod whose backlog is over `MAX_BACKLOG_VOXELS` is not called
  until it drains.
- Set `MOD_VOXELS_PER_TICK` by measurement: time a Voxel through `set_voxel`
  with a relight (the `dev` gallery's builders are the harness), choose the
  number that keeps the worst main-thread tick where phase 5 left it, record the
  table, and compare the worst tick with a filling Mod against none using the
  interleaved method of the earlier tickets, load average beside each figure.

## Tests

- A fixture Mod returns a `fill` into a held, lit Chunk: at quiescence the
  Voxels are there, the Chunk's mesh was redelivered, light is correct (equal to
  a fresh solve of the same Voxels), and the level-1 cover was invalidated.
- A fill of 32 768 Voxels takes `ceil(32768 / MOD_VOXELS_PER_TICK)` ticks and a
  second Mod's small `place` lands within a tick of its own call (round-robin).
- A `place` into an unloaded Chunk is counted rejected, the Mod's next
  `TickContext.last` says so, and nothing else is affected.
- A shaped `place` past `MAX_SHAPED_PER_CHUNK` is rejected by `set_voxel`'s own
  check, not a second one.
- A Mod that traps is disabled, its queued Commands still apply, a second Mod
  continues, and `ModDisabled` is written once.
- Player Edits reach the next `TickContext`; a Mod's own Commands do not.
- A Mod over the backlog cap is not called, and is again once drained.
- Privacy: no `pub` mutator outside `crate::server`, and no `client/` file
  mentions `PendingCommands`, `ModJobs` or `server::mods`.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- The `MOD_VOXELS_PER_TICK` table and the worst-tick comparison are in Comments.


## Comments

- `ModJobs` replaces idle retention with load-order slots. Each slot owns an
  idle instance or its worker Task; its Mutex exists only because Stores are
  Send but not Sync. Exclusive ECS access uses `get_mut`, and execution owns
  the instance outside the Mutex. Jobs capture owned events, the Voxel size
  and an Arc of immutable tables; they never capture the ChunkStore.
- Landing polls once in `FixedPreUpdate`, before player Edits. The private
  queue drains immediately after `apply_pending`; dispatch is in
  `FixedPostUpdate`. A fill retains the shared `BoxCursor`, not expanded
  Voxels. Each attempted Voxel consumes budget, including a rejection.
- A held but unlit Chunk waits at its Mod's FIFO head, without consuming
  budget or preventing other Mods from draining. This follows the gallery
  builder's rule: initial bulk light must settle before incremental Edits.
- Every returned list passes the shared pure `validate` before queuing.
  Ordinary validation failures reject the whole batch, add
  `BatchReport::refused`, and preserve the instance. Execution and post-lift
  size faults disable once; queued work survives. Reports accumulate until
  the next dispatched call, including while backpressure pauses the guest.
- The player-only history holds 1024 events. It drops oldest events and
  counts them globally; each Mod has its own cursor and missed-event count.
  A running or backpressured Mod consumes neither events nor reports.
- The dev timer now covers `FixedFirst` through `FixedLast`, including Mod
  draining and gallery building. `worst_tick_ms` now means one individual
  fixed tick; frame totals still sum catch-up ticks for the frame-cost
  figures. The previous metric both excluded pre/post work and summed ticks
  in a frame, so old phase-5 absolute figures are not directly comparable.

### Budget measurement

Apple M5, 10 cores, dev opt-level 1, three async workers. Five interleaved
rounds with rotating order, a real WASM fill batch, the shipped 1178-Voxel
shape/translucency gallery plans and their normal builders. Each run settles
its initial light before timing 160 ticks; every write uses `set_voxel`,
including relight. Uptime is recorded before and after every process. The
runner waits for one-minute load below 12 and discards a run crossing 12.
Compilation was finished before the sweep. No sweep run crossed that limit.

Figures are medians of each run's worst tick/drain, mean us per attempted
Mod Voxel, and whole-process user plus system CPU seconds. CPU includes the
same initial gallery/fixture setup in both variants. Different caps reach
different portions of the fill: per-Voxel means are not a constant cost to
extrapolate, and CPU totals are not equal-work throughput comparisons.

| cap | tick ms | drain ms | us/Voxel | CPU s | load 1m |
| --- | --- | --- | --- | --- | --- |
| 0 | 1.627 | 0.000 | - | 0.641 | 9.52–10.06 |
| 4 | 2.292 | 0.546 | 35.16 | 0.869 | 9.52–9.92 |
| 8 | 1.819 | 0.632 | 28.36 | 0.836 | 9.52–9.92 |
| 16 | 2.331 | 0.935 | 14.15 | 0.817 | 9.52–10.44 |
| 64 | 3.045 | 2.917 | 4.16 | 0.862 | 9.52–9.92 |
| 256 | 10.672 | 10.585 | 1.84 | 0.838 | 9.52–9.92 |

The live cap-8 calibration's first three clean paired rounds showed a median
worst tick of 7.3 ms without a Mod and 11.3 ms with one, and process CPU of
54.117 versus 61.628 s (load 6.34–11.22). Its fourth pair is excluded because
the no-Mod run crossed load 12.85. Both members of that pair are excluded
from the comparison, with their raw records retained. This prompted a lower
budget sweep, rather than accepting 8 on the headless figures alone.

Five more interleaved headless rounds, with the same gallery and 160-tick
procedure, compared 0 and 2:

| cap | tick ms | drain ms | us/Voxel | CPU s | load 1m |
| --- | --- | --- | --- | --- | --- |
| 0 | 3.543 | 0.000 | - | 0.797 | 10.20–10.43 |
| 2 | 2.099 | 0.296 | 30.33 | 0.965 | 10.43–10.43 |

`MOD_VOXELS_PER_TICK` is 2. Its measured drain leaves more room for the
gallery and streaming, whereas 256 costs most of a 15.625 ms tick by itself.
A full 32768-Voxel fill takes exactly 16384 otherwise-uncontended, lit ticks,
about 256 seconds at 64 Hz. Fairness lets a second Mod's place land in the
first drain tick. This trades fill latency for main-thread cost; a bulk
relight path remains a follow-up and must preserve `set_voxel`'s invariants.
The lower sweep's raw evidence is
`target/phase7-runs/t07-sweep-t9t9r8j_/results.json`.

Sweep raw logs, uptime and per-process CPU:
`target/phase7-runs/t07-sweep-nmdspsey/results.json`. All samples, including
outliers, are retained: the individual worst ticks ranged from 1.565 to
3.395 ms without a Mod (load 9.52–10.06), and 1.763 to 7.071 ms at cap 8
(load 9.52–9.92). These are measurements, not a hard wall-time guarantee.
The initial live calibration is discarded because its two variants had
unequal durations and used the old frame-sum tick metric. The sandboxed
launch that could not see a GPU contributes no timing result.


### Tests and acceptance review

- The real WASM fixture clears a held, lit region and places an emitter.
  At quiescence the test checks every Voxel, mesh redelivery, propagated
  emission, exact equality with a fresh light solve and cover invalidation.
  A separate integration test drains a validated fill under a held level-1
  cover and checks regenerated Voxels and redelivered coarse geometry.
- Full fills resume across ticks and take `ceil(32768 / budget)` ticks when
  uncontended. The second Mod's small place applies in the first drain;
  unlit FIFO heads leave other Mods free to progress.
- Unloaded placements and the existing shaped cap count as rejections in the
  next guest report. Invalid content refuses the whole batch and retains the
  guest; an oversized lifted result is fatal and retains prior queued work.
- A trapping guest disables and emits `ModDisabled` once, drains its remaining
  fill, and leaves a second guest calling. Backlog pauses at over 262144
  Voxels, resumes after draining, and preserves pending events and reports.
- Successful player Edits carry metre positions and actual before/after
  Voxels. Refused player intents and Mod Commands add no events. A schedule
  test pins player apply before Mod drain; history tests pin independent
  cursors, bounded retention and exact drop counts.
- Source privacy checks reject Client mentions of `PendingCommands`,
  `ModJobs` or `server::mods`, and pin the Server-private queue visibility.
  No queue mutator is public outside the runtime module.


### Verification evidence

- `cargo fmt --all` and `npm run fmt` completed. Prettier ignores `.scratch`;
  this ticket and the appended Outcome are hand-wrapped at 80 columns.
- `./scripts/cargo-ci` is fully green, including clippy across all targets and
  features, the default build, guests, workspace tests, bench compilation,
  crate boundaries and cargo-deny advisories/licenses/bans/sources. The deny
  cache ran with the required host permission; it was not skipped.
  Log: `target/phase7-runs/t07-ci-final.log`.
- `RUSTFLAGS='-D warnings' cargo test -p tesserae --features dev --locked`
  passed 265 tests. The one ignored test is the manual measurement harness,
  executed separately in the recorded sweeps. Log:
  `target/phase7-runs/t07-dev-tests-final.log`.
- Automated dev captures now ignore manual camera controls. Earlier captures
  showed unintended sideways movement while other browser work was active;
  those calibrations cannot establish a stable budget. The final capture
  runner waits for a quieter starting load and records every process's
  uptime, user/system CPU and screenshot path.


### Final live comparison and limitations

Same final dev binary, default seed and gallery, three async workers, Metal
with host permission, offscreen rendering, 30-second flight gate followed by
one second of flight and one second of settling. Two interleaved pairs ran
in ABBA order. The final runner waited for one-minute load below 6 and would
discard a run above 9; all four final runs passed that stricter gate. Each
row has its own before/after uptime and process CPU record. No guest was
disabled; both filling runs logged backpressure once and kept draining.

CPU is whole-process user plus system seconds. Flight frames are included
because presentation pacing varied during these runs: these CPU totals are
observations of the live application, not equal-render-work throughput.

| pair | cap | worst tick ms | CPU s | flight frames | load 1m |
| --- | --- | --- | --- | --- | --- |
| 0 | 0 | 3.7 | 115.955 | 121 | 5.12→6.12 |
| 0 | 2 | 5.3 | 121.856 | 125 | 5.04→6.07 |
| 1 | 2 | 3.6 | 87.453 | 60 | 4.79→5.52 |
| 1 | 0 | 6.1 | 116.002 | 128 | 5.52→6.74 |

The filling Mod's observed individual worst ticks are within the control
range in this pass. This supports the conservative two-Voxel budget; it does
not prove a universal latency bound or reproduce the old, narrower phase-5
metric. Background VM/browser work and changing presentation pacing prevent
stronger conclusions from the earlier live calibrations. The cap-2
calibration before manual-input isolation is discarded for choosing the live
budget;
those records, and the high-load runs, remain available for inspection.

Final uptime/CPU/capture records and PNGs:
`target/phase7-runs/t07-capture-fg5pxfxt/results.json`.
The paired screenshots are `r0-v0.png`, `r0-v1.png`, `r1-v1.png` and
`r1-v0.png` in that directory. The filling screenshot was visually inspected.
The repeatable native sweeps, live comparison, all ticket tests and full CI
are complete. The remaining performance cost is explicit: a full fill takes
about 256 seconds, and continuous fills also cause streaming/mesh work.

For 08/09: the pre-App `LoadedMods`/Trust loading API remains the entry point;
the plugin consumes its instances into `ModJobs`. `ModInstance::id()` is a
read-only Manifest id for fault names. Guests still implement the existing
WIT tick contract; no new import or Client queue access exists. Treat reports
as counts accumulated since the last dispatched call, because older queued
work can still be draining. The 49.0.4 security patch remains locked.
