# 09: Calibrate responsive fluid and fire budgets

Status: done
Blocked by: 08

Spec: [decision 11 and measurement policy](../spec.md).

## What to build

Choose evidence-backed defaults that make ordinary fluid/fire interactions
visible while preserving player Edits, Mod fairness, streaming, and bounded
tick work. Resolve the named relight performance question explicitly.

## Acceptance criteria

- [x] Native benches cover idle/dry terrain, uniform lakes, flow/recession,
  waterlogging-heavy geometry, reactions, ignition/burning/extinguishing, and
  saturated continuation work. Track CPU, operations, and retained memory.
- [x] Controlled/live scenarios include a 16 m waterfall, a flat basin wider
  than water reach, four separated active outlets, a 16 m fuel chain with a
  barrier and extinguishing water, and a saturated frontier.
- [x] Use equal physical fixtures at 1.0, 0.5, and 0.25 m Voxel sizes. Report
  logical progress, source-to-visible latency, mutation/mesh progress, backlog,
  and complete Server tick distributions beside CPU and load figures.
- [x] Compare affected native hot paths with the pre-change evidence from
  ticket 01. CPU comparisons use equal work and interleaved orders; live
  unequal-presentation CPU is labeled as observation, not throughput.
- [x] Record uptime before and after every process and load beside every
  figure. Use unique paths, retain raw records, and discard distorted pairs
  from conclusions without concealing them.
- [x] Set count quotas from repeated quiet-load evidence, using 15.625 ms at
  64 Hz as the full-tick reference. Report control and fluid/fire distributions
  without claiming a universal wall-time bound from a count budget.
- [x] Show that ordinary interactions progress usefully, settled state is
  idle, saturated work stays bounded/fair, and continuous changes still draw.
  Do not preserve responsiveness by accepting unusably slow ordinary flow.
- [x] Evaluate the repeated incremental relight cost. A general bulk relight
  rewrite stays a named follow-up; if required to pass the accepted scenarios,
  present evidence and obtain an explicit prerequisite/scope revision before
  adding it or calling the phase complete.
- [x] Do not raise the phase 7 two-Voxel Mod drain cap or alter security
  patches as an implicit performance adjustment.

## Verification and completion

Native measurements, integration runs, and conclusions belong in Comments.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- This is an integration/measurement gate. A correct but stalled waterfall,
  unmeasured tiny quota, or skipped relight issue does not satisfy it.
- 2026-10-24: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- Measurement found three defects, fixed here with tests:
  - A fire change past the landing bound restarted its whole delay. The
    Server re-held only a flow timer for a deferred proposal, so a put-off
    ignition waited another 2 s and a put-off burn-out burnt another 8 s. At
    0.25 m an ignition wave is wider than 64 changes, and fire spread at half
    the 1 m rate (7.4 m in 28 s against 14 s). `flow::Timers::defer(&p,
    ready)` (replacing `hold`) holds the fire timer `fire::Fire::of(p)` reads
    back from the proposal, and the flow timer unless the change is a burn-out
    (which moves the fluid into its residue; flow decides again from it).
  - `start_step` built the step's Region per due Voxel (its 10-11 reads and,
    for fuel, a radius box of Chunks), about 1 us of main-thread time per due
    Voxel at dev opt-level: 4096 rescanned Voxels cost 7-13 ms. It now takes
    every simulated Chunk within `around` (the deepest read or heat radius,
    in Chunks) of each due Voxel's own Chunk, found once per Chunk. Chunks no
    evaluation reads change nothing it computes; staleness still compares
    only the Chunks a proposal reads.
  - `wake_fuel_around` looked up the store once per Voxel of its radius box
    (3375 at 0.25 m) for every heat-relevant landing; the Server's closure now
    keeps the last Chunk, so one lookup serves a span.
- `EMITTERS_PER_STEP` (4): a second landing bound for changes that start or
  stop block light (lava arriving or going, fire catching or going out),
  decided from the proposal's read and next emission, so deterministic. Each
  is an incremental relight of its light's reach, 100-1000 times an ordinary
  change. `choose` keeps its turn and cursor order; once the step's emitters
  are spent a Chunk's turn takes its next change that moves no light, so
  water is never held behind fire.
- Ticket 08's remaining starvation case is bounded: `MeshState::spoilt`
  counts a Chunk's solves dropped in a row for reading what changed, and
  after `SPOILT_BEFORE_ON_TICK` (4) its next solve is made on the tick, where
  nothing can change under it (at most one a tick; `apply_solve` is the one
  landing path for both). A writer changing the plane a neighbour's solve
  reads on every tick for ever no longer keeps that light, or the meshes
  waiting on it, from landing.
- Defaults: `LANDED_PER_STEP` 64 to 256; `EMITTERS_PER_STEP` 4 (new); agenda
  16384 + 4096, 4096 rescanned a step, 1024 evaluations a call, 4 jobs, the
  journal and edited sets 4096 unchanged. Water and lava tuning unchanged;
  `FluidDef::WATER`'s "pending measurement" note is replaced.
- `crates/tesserae-core/benches/fluids.rs` (new, wall clock like the others):
  the kernel driven as the Server drives it over a Region, at 1.0, 0.5 and
  0.25 m: rescans of a lake shore, dry terrain and a uniform lake's faces, a
  settled step, a 16 m waterfall falling, spreading and receding, water
  through a floor of slabs and posts, lava meeting water, a 16 m wood bar
  burning and a burning bar put out, a four-Chunk rescan past the agenda's
  capacity, and eviction from a full agenda. Throughput is evaluations; each
  scenario prints its work, peak agenda, timers, proposals per step, overlay
  bytes, and the changes a landing bound of 64/256/1024/4096 would defer.
- `server/fluids/tests/measure.rs` (new, `#[ignore]`, `dev`): the real
  `mods::drain`, `simulate`, `stream` and `stream_lod` systems on
  single-threaded schedules over flat generated terrain (24 m view, coarse
  levels to 256 m, real streaming and LOD budgets, three async workers, the
  demo's real tables), ticked at 64 Hz. Per tick it records main-thread wall
  and thread CPU per part, logical step, evaluations, landed, deferred, stale,
  parked, scanned, agenda, rescans, timers, meshes and level-1 meshes
  delivered, dirty and unlit Chunks, jobs and Mod Voxels applied; per probe
  Voxel, when its state changed and when a delivered mesh first drew it (a
  quad overlapping it) at level 0 and level 1. Scenarios: `control`,
  `waterfall`, `basin`, `outlets`, `lava`, `fire`, `saturated`, and
  `relight` (one World change's cost by kind). Every scenario runs two Mod
  queues of 40 placements and a player stone every 64 ticks.
  `streaming::tests::Harness::with_world` and `add_stream_systems` serve it.

### Verification

- Test functions: core lib 317 (316), meshing 66, Mods 89 (three ignored),
  `tesserae` binary 347 (343; two ignored: the earlier one and the new
  measurement); workspace 819 (814). Full `scripts/cargo-ci` passed on the
  tree this commit records.
- New tests, each failing before its fix and mutation-checked by removing
  it: `flow::tests::fire::a_change_put_off_by_the_landing_bound_lands_the_
  step_after` (a put-off ignition, burn-out and flow change land the step
  after under a landing bound of one; without the fire hold the second
  ignition came 128 steps late); `fluids::tests::fire::a_fire_change_past_
  the_landing_bound_lands_the_next_step` (the same through the Server: the
  second wood caught 128 steps after the first, now within 2);
  `fluids::tests::emitters_past_their_bound_wait_while_other_changes_land`
  (2 + 2 + 1 lamps over three steps under a bound of two, all six water
  changes in the first); `streaming::tests::progress::a_solve_spoilt_every_
  tick_by_its_neighbours_plane_still_lands` (water toggled every tick on a
  lantern-lit boundary plane: the east Chunk's light never landed in 200
  ticks; now after four spoilt solves, its mesh follows, and the settled
  light equals a fresh `light_cells` sweep; failing again with the bound set
  to `u8::MAX`).
- Changed: `reactions_past_the_landing_bound_land_whole_in_later_steps`'s
  unbounded run lifts the emitter bound too, since each of its nine coolings
  puts out lava's light.
- Every other test passes unchanged, including ticket 03's saturation,
  determinism and division tests under the new region build and choice.

### Measurement method

- Server ticks: the `measure.rs` harness above, test binary at the `dev`
  profile (opt-level 1 for the workspace crates, as `cargo dev` runs it), one
  process per scenario and Voxel size, `uptime` before and after each in
  `runs.log`, raw per-tick CSV and summaries kept. Main-thread time is
  wall and thread CPU (`CLOCK_THREAD_CPUTIME_ID`) of the three schedules on
  single-threaded executors; CPU is the figure conclusions use, wall is
  reported beside it. 15.625 ms (64 Hz) is the reference.
- Fixtures, equal in metres at 1.0, 0.5 and 0.25 m, on flat terrain (floor
  at 24 m, no caves, smoothing or lakes), all inside the mesh extent:
  `waterfall` (a source 16 m up; removed once settled), `basin` (a source on
  the floor, wider than reach both ways; removed), `outlets` (four 16 m
  falls 12 m out on the axes, 24 m apart; removed), `lava` (demo lava 4 m
  up; removed), `fire` (two 16 m wood bars 1 m square 12 m apart, lit at the
  west end; bar A cut at its middle by a stone wall 1 m thick, 3 m tall, 5 m
  wide; water poured on bar B 10 s in), `saturated` (a dam: a wall of
  sources 16 m wide and 4 m high pouring out on all sides, beside a 12 m
  square wood floor 1 m thick lit along one edge), and `control` (20 s of
  the background Edits alone). A scenario runs until the simulation is idle
  and every probe has been drawn, then 64 ticks in which the evaluations are
  counted (all 0).
- Native: thread-CPU criterion (the uncommitted patch of ticket 01, applied
  to all four core benches), `--warm-up-time 1 --measurement-time 2`.
  Pre is ticket 01's 2ae7399 binaries; post is this tree.
- Raw data (not committed): Server runs in
  `/private/tmp/tesserae-p8-t09.b/` (`final/r1..r3` all scenarios at the
  chosen bounds with 8 emitters, `e4/r1..r3` fire, lava and saturated at 4
  emitters, `sweepL` the landing-bound sweep, `sweep25*` before the fixes,
  `relight1` and `final/r*/relight-*.txt`, `summary.md` per set); native in
  `/private/tmp/tesserae-p8-t09.c/` (`ab.log`, `criterion/`, `ab-*.out`,
  `fluids-r*.out`, `ab-summary*.md`).
- Discarded, kept on disk: `sweep25b`'s saturated and outlets runs (load 29
  to 48 from another session's build; ticks fell behind steps); `final/
  killed-r2` (cut off by a host restart mid-round); `e4/r3` for wall figures
  (load 6 to 19: walls to 51 ms against CPU of 7 ms, ticks behind steps);
  native round 1's post side (load 9 to 18). `pmset -g log` shows no sleep
  during any kept run; the host slept from 03:07 (where the killed round
  stops) to 14:44, between sets, and 01:04-02:55 after `sweepL`.

### Server tick distributions (dev profile, chosen bounds)

Main-thread thread CPU per tick, three clean rounds each: median of the
rounds' p50 and p99, maximum of their maxima; wall maximum beside it. Load
(one-minute average) during kept runs was 1.5 to 7.6. Bounds: 256 landed
and 4 emitters a step (fire, lava, saturated from `e4/r1,r2`, the rest from
`final/r1..r3`, where the emitter bound is idle: water emits nothing).

| scenario | v (m) | ticks:steps | p50 | p99 | max | wall max | > 15.625 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| control | 1 / 0.5 / 0.25 | 1344:1343 | 0.04-0.09 | 0.20-0.27 | 0.40 | 0.81 | 0 |
| waterfall | 1 | 836:835 | 0.04 | 1.09 | 3.46 | 5.88 | 0 |
| waterfall | 0.5 | 820:819 | 0.07 | 2.59 | 6.74 | 6.74 | 0 |
| waterfall | 0.25 | 815:814 | 0.15 | 2.10 | 5.10 | 5.10 | 0 |
| basin | 1 | 420:419 | 0.06 | 0.88 | 2.83 | 5.18 | 0 |
| basin | 0.5 | 420:419 | 0.06 | 2.24 | 7.47 | 10.21 | 0 |
| basin | 0.25 | 423:422 | 0.11 | 2.19 | 5.39 | 5.39 | 0 |
| outlets | 1 | 837:836 | 0.06 | 1.87 | 6.14 | 6.14 | 0 |
| outlets | 0.5 | 823:822 | 0.07 | 4.22 | 9.86 | 9.89 | 0 |
| outlets | 0.25 | 886:885 | 0.16 | 2.66 | 4.64 | 4.64 | 0 |
| lava | 1 | 1397:1396 | 0.04 | 1.60 | 7.13 | 10.29 | 0 |
| lava | 0.5 | 1332:1331 | 0.06 | 2.59 | 7.65 | 9.91 | 0 |
| lava | 0.25 | 1337:1336 | 0.11 | 4.42 | 11.17 | 11.17 | 0 |
| fire | 1 | 1538:1537 | 0.04 | 1.51 | 4.71 | 4.71 | 0 |
| fire | 0.5 | 1672:1671 | 0.05 | 2.68 | 7.26 | 7.26 | 0 |
| fire | 0.25 | 1699:1698 | 0.10 | 1.97 | 3.34 | 3.34 | 0 |
| saturated | 1 | 2058:2057 | 0.04 | 1.51 | 6.13 | 6.13 | 0 |
| saturated | 0.5 | 2322:2321 | 0.06 | 1.82 | 4.20 | 4.69 | 0 |
| saturated | 0.25 | 2513:2483 | 0.84 | 3.41 | 8.67 | 8.67 | 0 |

- Streaming's own part stayed at p99 0.2-0.3 ms throughout (the World is
  streamed in before timing); the Mod drain applied all 80 Voxels at two a
  tick in every run, and every player Edit landed on its tick (asserted).
- The 0.25 m saturated run's ticks lead its steps by 30: steps of up to 9399
  evaluations take 10 worker calls, 4 in flight. Every other run keeps one
  step a tick.

### Logical progress and source-to-visible latency

Logical steps from the event to the probe's state; it was drawn at level 0
in the next tick (latency 1 tick, 16 ms) unless noted. Identical in every
round: progress is deterministic, only its tick cost varies.

| probe (from) | 1 m | 0.5 m | 0.25 m | physical |
| --- | ---: | ---: | ---: | ---: |
| waterfall foot (source) | 128 | 128 | 128 | 16 m / 8 m/s = 128 |
| waterfall 3 m out | 176 | 176 | 176 | + 3 m / 4 m/s = 176 |
| waterfall foot dry (removal) | 384 | 368 | 370 | 16 + 3 m receding |
| basin 3 m / 6.5 m out | 48 / 96 | 48 / 104 | 48 / 104 | 48 / 104 |
| basin 3 m dry | 112 | 112 | 115 | |
| four outlets, each foot | 128 | 128 | 128 | 128 |
| four outlets foot dry | 384 | 368 | 432-438 | |
| lava foot / 2 m out | 170 / 342 | 171 / 345 | 170 / 347 | 171 / 342 |
| fire bar A 4 m / 7.5 m | 384 / 896 | 515 / 902 | 528 / 1040 | 1 m per 2 s |
| fire bar B front out (water) | 113 | 113 | 113 | |
| saturated flood 6 m out | 104 | 100 | 124 | 96 |
| saturated fire far edge | 1414 | 1545 | 1659 | 11 m per 2 s |

- The fire chain spreads one metre per ignition delay at every size, to the
  one-Voxel hop of the 1 m Voxel (3 hops for "4 m" at 1 m, 4 at 0.25 m).
  The barrier held at all sizes (bar A's wood past it never caught) and the
  water put bar B's front out 1.8 s after it was poured and kept the rest
  wood. Before the deferral fix it took 1152 / 1792 steps at 0.25 m.
- Under saturation the bounds slow work deterministically: four outlets
  recede in 432-438 steps at 0.25 m against 370 for one (122848 deferrals
  over the run; 820 at the old bound of 64); the flood front at 0.25 m is
  20 steps behind its 1 m time; the wood floor's wave, 48 ignitions wide at
  0.25 m, lands 4 a step and reaches its far edge 17% later than at 1 m.
  Nothing stalls: each probe is reached, each run settles, the agenda peaked
  at 9399 (no overflow), and evaluations after settling were 0 in every run.
- Visible lag: lava drying is drawn 56-83 ticks after the state at 0.25 and
  0.5 m, and water 5-17 ticks after receding (`edge_2m_dry`, `foot_dry`):
  the light of each drained Voxel moves its neighbours', the mesh waits for
  settled light, and a mesh that lands is followed by the next. Level-1
  covers follow within 2 ticks. No mesh in the mesh extent stayed dirty
  more than 4 ticks in any run.

### Choosing the bounds

- `LANDED_PER_STEP`, swept at 0.25 m before the emitter bound (`sweepL`;
  64 / 128 / 256; load 2.1-7.0): four outlets receded in 818 / 562 / 434
  steps with tick CPU max 2.65 / 3.07 / 3.50 ms; a single waterfall in 427 /
  378 / 370; the saturated dam's flood reached 6 m in 329 / 198 / 124 steps.
  With region building per Chunk a landed water change costs about 10 us on
  the tick, so 256 bounds a water step near 3 ms, and recession at 0.25 m is
  no longer several times slower than at 1 m. Deferral also costs worker
  evaluations: the outlets ran 1.34 M at 64, 0.51 M at 256.
- `EMITTERS_PER_STEP`: one emitter change relights its light's reach. In a
  dark room at dev profile it costs (us, p50 / max) lava appearing 22 / 83 at
  1 m, 125 / 196 at 0.5 m, 235 / 337 at 0.25 m; fire catching 22 / 23, 83 /
  119, 135 / 179; water 0.4-2.4; a surface change 0.2 (35 when it grows the
  overlay's palette). Outdoors lava draining at 0.25 m cost 2.75 ms a Voxel.
  At 16 a fire step reached 12.6 ms; at 8 a lava step 13.75 ms (`final`);
  at 4 the worst is 11.2 ms (lava draining at 0.25 m) and fire 7.3 ms. The
  slowdown 4 costs is 8 steps on the fire chain's 4 m probe at 0.25 m.
- Unchanged, measured: the agenda (16384 + 4096) peaked at 9399 entries with
  2488 timers in the saturated run and never overflowed; the fire chain and
  the wood floor kept their timers well inside it (479 and 2488). Rescans of
  4096 a step cost about 8 ms of worker CPU per Chunk-sized scan in release
  (`fluids/rescan`: 265 ns an evaluation). 1024 evaluations a call is about
  0.3-0.5 ms of a worker in release (2-8 us for fuel, whose search is the
  costly part); 4 jobs keep steps to one a tick except the 9399-wide one.
  Eviction from a full agenda costs 132 us. The journal and edited sets at
  4096 coalesced only in the saturated set-up's 10000-Voxel build.
- A quota bounds work, not time: these ticks are under 15.625 ms on this
  host at the dev profile; a coincidence of a lava drain (11 ms) with a
  streaming worst tick (phase 7 saw 3.7-6 ms live) could take one tick
  past it, after which the fixed loop catches up rather than skipping. A
  release build runs these paths several times faster (native rows below).

### Relight (spec decision 11)

- Incremental relight is not a bulk-relight problem for water: a water
  change relights in 0.4-2.4 us, and 256 a step fit easily. Emitters are the
  costly case, 0.1-2.75 ms each, and the emitter bound keeps any step's
  relight work to four of them, with the measured slowdown above.
- No accepted scenario needs a bulk relight rewrite: every tick stayed under
  15.625 ms and every probe progressed at its physical rate or within the
  saturation slowdowns recorded. A bulk relight would cut the remaining
  cost (lava draining outdoors at 0.25 m, 11 ms a step at dev profile) and
  let the emitter bound rise; it stays the named follow-up. No scope
  revision is needed for this phase.

### Ticket 08's starvation case

- Bounded, not accepted: after four spoilt solves in a row a Chunk's light
  is solved on the tick (one a tick, a bulk solve's cost, 0.1-0.3 ms in
  release per `light_chunk`), so a writer changing a boundary plane every
  tick for ever delays its neighbour's light by four solves, not for ever.
  The test above pins it.

### Native benches (release, thread CPU)

`fluids` medians of the two clean rounds, per scenario run (one settle, or
place, settle, remove and settle):

| bench | 1 m | 0.5 m | 0.25 m | evaluations at 0.25 m |
| --- | ---: | ---: | ---: | ---: |
| waterfall_16m (fall, spread, recede) | 1.87 ms | 6.11 ms | 29.6 ms | 65800 |
| waterlogged_floor (slabs and posts) | 3.82 ms | 17.0 ms | 28.1 ms | 61143 |
| reaction_front (lava meets water) | 707 us | 3.21 ms | 18.6 ms | 14261 |
| fire chain_16m (lit end to ash) | 1.22 ms | 9.63 ms | 118.8 ms | 13378 |
| fire extinguish (4 m burning bar) | 1.21 ms | 4.53 ms | 31.6 ms | 32704 |

- Rescans of a whole Chunk: lake shore 8.7 ms, dry terrain 8.9 ms (32768
  evaluations, about 270 ns each); a uniform lake's faces 0.94 ms (5768);
  a settled step 138 ns; four Chunks past the agenda's capacity 37 ms
  (131072); forgetting one Chunk of a full agenda 135 us.
- Retained at peak (from the bench's `ops` lines): the 0.25 m waterfall 3722
  agenda entries, 900 timers, 900 proposals in one step, 172 KB of overlay
  over 12 Chunks; a landing bound of 64 would have deferred 8207 changes
  over 35 steps, 256 defers 4375 over 14, 1024 none.
- Fire is the costly kernel path: 8.9 us an evaluation at 0.25 m (142342
  open-space nodes searched for 9008 fuel evaluations), against 0.3-0.5 us
  for flow.

### Hot paths against ticket 01's pre-change baseline

Pre is 2ae7399 (ticket 01's binaries), post this tree; thread CPU, rounds
2 and 3 (load 3.5-5.7; round 1's post side ran at 9-18 and round 4 at 6-12,
both kept on disk and moving no conclusion). Minimum and median of
criterion's per-round medians:

| bench | pre min | post min | Δ min | Δ med |
| --- | ---: | ---: | ---: | ---: |
| greedy_mesh/terrain | 346.5 us | 405.9 us | +17.1% | +31.1% |
| greedy_mesh/solid | 339.7 us | 412.9 us | +21.5% | +51.1% |
| greedy_mesh/glass_room | 263.9 us | 302.6 us | +14.6% | +16.8% |
| greedy_mesh/terrain_water | 398.7 us | 453.7 us | +13.8% | +20.2% |
| greedy_mesh/all_stairs | 3.62 ms | 3.56 ms | -1.5% | +5.2% |
| mesh_chunk/straddling | 411.0 us | 445.4 us | +8.4% | +5.8% |
| mesh_chunk/shore | 419.3 us | 467.4 us | +11.5% | +15.7% |
| mesh_chunk/declined_air | 12.1 ns | 22.6 ns | +87.6% | +81.4% |
| neighborhood_isolated | 25.6 us | 39.9 us | +55.9% | +60.6% |
| light_chunk/straddling | 182.9 us | 206.2 us | +12.8% | +21.7% |
| light_chunk/straddling_water | 132.9 us | 158.4 us | +19.2% | +25.8% |
| light_chunk/cave_mouth | 102.9 us | 129.9 us | +26.2% | +31.9% |
| light_chunk/leaves_canopy | 171.6 us | 196.5 us | +14.5% | +23.9% |
| shape_register_fence | 648.0 us | 1.07 ms | +65.2% | +94.1% |
| shape_table_builtin | 4.38 ms | 6.02 ms | +37.3% | +56.8% |
| lod_downsample/worldgen | 257.0 us | 209.2 us | -18.6% | -17.3% |
| lod_downsample/stairs | 611.4 us | 490.3 us | -19.8% | -22.1% |
| lod_mesh_coarse/surface_level_1 | 409.3 us | 435.0 us | +6.3% | +5.6% |
| lod_mesh_coarse_lit/surface_level_1 | 471.2 us | 499.3 us | +6.0% | +2.0% |
| lod_coarse_chunk/surface_level_1 | 2.11 ms | 2.01 ms | -5.1% | +1.3% |
| collide/fall_terrain | 153.1 ns | 152.5 ns | -0.4% | +3.9% |
| collide/ray_terrain | 474.4 ns | 473.7 ns | -0.1% | +0.7% |
| worldgen/straddling | 169.0 us | 168.7 us | -0.2% | +3.0% |
| snapshot/make_mut_shared | 307.1 ns | 314.0 ns | +2.3% | +20.1% |

- Over all 93 rows the minimum moved -19.8% to +87.6%, median +2.7%;
  `ab-summary-r23.md` has every row. Collision, worldgen and snapshots are
  unchanged; downsampling is faster (ticket 08's per-octant split).
- **The phase made dry meshing and bulk light 10-30% slower** on the worker
  paths every streamed Chunk takes, and shape registration 40-90% slower
  (a startup cost, 6 ms for the built-in table). This ticket is not the
  cause: against ticket 08's own final meshing binary (585ae2c, two
  interleaved rounds, load 4.2-4.7) every meshing and light row moved within
  -5.5% to +7.2% with no direction. Tickets 02 to 07 recorded "no bench was
  rerun"; the cost is theirs: `light_cells` reads the overlay and the wet
  light parameters per Voxel, `Neighborhood` scans seven overlays and keeps
  fluid side arrays, and registration flood-fills every variant's voids.
  It is worker CPU, not tick time, and it is not what the Server-tick gate
  measures; a dry-Chunk fast path (a uniform-`NONE` overlay taking the old
  light and mesh paths) is the obvious follow-up, recorded for ticket 10 and
  the phase PR rather than added here.

### Deviations, limits and what later tickets inherit

- Server figures are at the `dev` profile (opt-level 1), the build `cargo
  dev` runs, so they overstate a release build; the native figures are
  release. The live renderer was not measured: the Server tick is, on its
  own schedules, and the Client's mesh uploads are outside it. Live
  unequal-presentation CPU was not used for any conclusion.
- The landing-bound sweep ran one round per value; the chosen values were
  then run three rounds (water) and two clean rounds (emitters).
- The emitter bound slows a wide fire front at fine Voxel sizes under
  saturation (17% on the 12 m wood floor at 0.25 m); the fire chain's 1 m
  bars lose 8 steps in 1040. Both settle as unbounded runs do.
- The emitter bound counts changes, not relight size: a lava drain outdoors
  at 0.25 m is the measured worst (11.2 ms a step, dev profile). A bulk
  relight is the follow-up that would cut it.
- `start_step` now snapshots every simulated Chunk within the read radius of
  a due Voxel's Chunk, more Arcs than the Voxels read; a Chunk no evaluation
  reads cannot change a result, and staleness is still per read.
- A Chunk solved on the tick after four spoilt solves costs one bulk solve
  of main-thread time, at most one a tick.
- For ticket 10: the bounds, their evidence and the dev-profile caveat go in
  the docs' measured budgets; the hot-path regression above needs a decision
  before the phase PR; `EMITTERS_PER_STEP`, deferral holding timers, and the
  on-tick solve after repeated spoils are new behaviour for CONTEXT.md /
  architecture. No live capture was taken here.
