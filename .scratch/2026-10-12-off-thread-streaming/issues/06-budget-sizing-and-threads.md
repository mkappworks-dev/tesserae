# Size the caps and the thread count from measurement

Status: done
Blocked by: 05

Every number so far is a placeholder. Spec decision 6 and "What this assumes
about per-Chunk cost".

## What to build

- Measure on the ticket 01 harness, five runs per configuration: async compute
  threads at 2, 3 and 4 (set through `TaskPoolPlugin` options in `main.rs` if
  one wins); in-flight caps over a sweep per stage; landing caps over a sweep
  for mesh.
- Measure the Client's cost per delivered mesh (`chunk_mesh_to_mesh` plus
  `Assets<Mesh>::add`), which is what the mesh landing cap actually bounds. If a
  full landing cap of meshes in one frame costs more than a couple of
  milliseconds, say so: it is the next thing to move off the render thread, and
  a loose end the spec already names.
- Set the defaults. Write `StreamingBudget`'s doc comment in the same form the
  old one had: what was measured, on what, what it implies, and what the number
  does *not* bound. State what a bad value costs (fill latency or stale
  priority, not frame time) and what the sizing assumes about per-Chunk cost,
  pointing at the spec's four assumptions.
- Update DESIGN.md's "Per-ring LOD chunk budget" open item: the three budgets
  are no longer sized against each other, and what it measures has changed
  meaning.

## Acceptance criteria

- The sweep tables are in Comments, with the machine and load.
- The doc comment contains no number whose reason is gone (the old "relight is
  half of generation" paragraph is rewritten or removed).
- The staleness window (`cap x mean cost / threads`) for the chosen defaults is
  computed from measured costs and stated.
- `./scripts/cargo-ci` passes.

## Comments

**The harness changed first, because frame time could not answer the question.**
Under vsync a frame is the refresh interval, and the display moved between 60,
120 and 144 Hz on the same commit within an hour (flight median 16.7 ms, then
8.3, then 2.1). So the capture now presents without waiting for the display
(`TESSERAE_SCREENSHOT_VSYNC` restores it), and reports what streaming costs the
main thread directly: `MainThreadCost` (`dev` only) times every run of the tick
system and every Client mesh upload. Even uncapped, some runs still present at
the display's cadence, so frame figures stay in the report but the cost figures
are the ones to read. `TESSERAE_SCREENSHOT_FLY_AFTER_S` makes the flight start
at the same time since launch for builds whose fills end at different times, and
`fill_s` is now when the Server settled, not when the flight began.
`TESSERAE_BUDGET`, `TESSERAE_SETTLE_GATE` and `TESSERAE_ASYNC_THREADS` (all
`dev`) let a sweep run without a rebuild.

**Sweeps**, Apple M5, debug at `opt-level = 1`, five runs each, medians. In
flight caps are generate / relight / mesh; landing caps are 64 / 256 / 32 unless
noted.

- placeholder caps (48/24/24, 48/12/12): shown 7.78 s; filled 8.09 s; worst tick
  1.1 ms; worst frame, fill 45.6 ms
- the same, gate off: shown 1.23 s; filled 8.18 s; worst tick 0.3 ms; worst
  frame, fill 19.5 ms
- in flight 16/16/16: shown 5.88 s; filled 6.11 s; worst tick 1.2 ms; worst
  frame, fill 44.5 ms
- in flight 32/32/32: shown 3.07 s; filled 3.21 s; worst tick 0.5 ms; worst
  frame, fill 27.2 ms
- **in flight 64/128/128 (chosen)**: shown 1.54 s; filled 1.57 s; worst tick 0.7
  ms; worst frame, fill 8.5 ms
- the same, gate off: shown 1.11 s; filled 1.63 s; worst tick 0.6 ms; worst
  frame, fill 15.9 ms
- the same, mesh landing 8 / 64: shown 2.54 / 1.40 s; filled 2.54 / 1.69 s;
  worst tick 0.5 / 1.5 ms; worst frame, fill 12.3 / 14.3 ms
- the same, 2 / 4 / 6 async threads: shown 1.55 / 1.36 / 1.54 s; filled 1.68 /
  1.63 / 1.54 s; worst tick 0.6 / 1.3 / **12.7** ms; worst frame, fill 19.7 /
  18.1 / 10.2 ms

- **The caps are a fill-latency knob, not a frame-time one.** Cutting them from
  128 to 16 moves the fill from 1.6 s to 6.1 s and the worst tick not at all.
  That is the trade the spec predicted. They stop mattering past about 64 / 128
  / 128: the workers, not the cap, are then the limit.
- **Three async threads stay.** Two is within noise on the fill. Four starts to
  hurt the main thread (worst tick 1.3 ms, up to 4.6), and six is a disaster
  (12.7 ms): six workers on ten cores take the cores the render and ECS threads
  need. No change to `main.rs` for release; the override is `dev` only.
- **The settled gate stays on.** At placeholder caps it looked ruinous (nothing
  for 7.8 s) because the relight backlog was the long pole; with the backlog
  gone it costs a Chunk's first appearance 0.2-0.4 s and saves more than half
  the mesh work (1111 meshes against 2427), with the fill the same. Ticket 05's
  rule ("keep it unless it costs fill time") is met. The worst-frame column does
  not support more than that: the chosen configuration's worst frame during the
  fill read 8.5 ms in one batch and 20-22 ms in the final interleaved runs, so
  frame figures between batches cannot be compared and the gate's apparent
  halving of it is not established.
- **Client upload is not the limit.** 6-12 us a mesh, so the mesh landing cap of
  32 is 0.4 ms. It is held at 32 because 64 doubled the worst tick to 1.5 ms for
  no gain in fill, and 8 slowed the fill to 2.5 s. This answers the spec's loose
  end: moving the upload off the render thread is not worth doing.
- The staleness window for the chosen caps, from the release benchmarks, if
  every job were a surface-crossing Chunk: 64 x 98 us + 128 x 139 us + 128 x 342
  us = 68 ms of work, 23 ms across three workers. About a tick and a half; far
  less in practice, since most jobs are nanosecond early-outs.

`StreamingBudget`'s doc comment is rewritten with these numbers and without the
old "relight is half of generation" paragraph, whose reason (relight's backlog
costing nothing visible, as a frame-cost argument) no longer sizes anything. The
DESIGN.md open item on the per-ring budget is updated.

**Final A/B**, 20 runs, interleaved (baseline, branch, baseline, branch), flight
starting at 9 s on both, medians of two rounds of five:

- worst main-thread time in a tick: `main` before 21.8 / 27.0 ms; this branch
  0.6 / 0.7 ms
- worst streaming cost in one frame, flight: `main` before 9.4 / 13.5 ms; this
  branch 0.3 / 0.5 ms
- fill (nothing left to do): `main` before 7.24 s; this branch 1.52 / 1.57 s
- mesh extent all shown: `main` before 1.10 s; this branch 1.45 / 1.22 s
- worst frame during the fill: `main` before 24.3 / 32.1 ms; this branch 20.4 /
  22.5 ms
- worst frame during flight: `main` before 21.1 / 28.0 ms; this branch 22.5 /
  20.0 ms
- 99th-percentile frame during flight: `main` before 13.6 / 24.8 ms; this branch
  14.9 / 18.3 ms

The main-thread streaming cost fell by about 35x and the fill by 4.6x. The
wall-clock worst frame fell 16-30% during the fill and is level in flight, and
it did not fall further because what is left of a 20 ms frame is not streaming:
with 0.7 ms of streaming work in the worst tick, the rest is presentation and
the render thread, which this task does not touch. The brief's criterion ("lower
and flatter worst frame ... at the same or better fill time") is met for the
fill and met as "not worse" for flight, where there was nothing to win (ticket
01).
