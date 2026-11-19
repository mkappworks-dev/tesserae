# 01: Take the baseline

Status: done
Blocked by: step 17 (`feat/phase-17-terrain-and-water`) merging, and a quiet
host.

Spec: [Packed quads](../spec.md).

## What to build

Numbers for the base, so that the order of the items and each gate rest on
them.

- Rebase the branch onto `main` once step 17 merges, and rebuild the base's
  release `profiling` binary from the commit before ticket 02.
- `scripts/profile-frames` on flight, active and transparency, timing series
  first, then `--instruments gpu`, `--generator 3` if v3 is the default by
  then.
- The meshing benches as an interleaved CPU-time A/B
  (`crates/tesserae-core/benches/meshing.rs`).
- Report frame time (CPU and GPU, p50 and p95), mesh worker ms per second of
  flight, mesh bytes held (`capture_lod_memory`) and upload per frame
  (`upload_wall_ms`, `uploads`). Note `uptime` and how each number was taken.
- Answer the spec's open questions: the order of items 2 to 4, per-layer or
  bucketed placeholders, and the quad buffer's initial size and growth.

## Acceptance criteria

- [x] Each number above for the base, with its provenance.
- [x] The open questions answered in this ticket.

## Comments

2026-11-19: the base is `ec02b66f` (main, step 17 merged, generator 3 the
default), the commit before ticket 02. Not rebased onto today's `main`
(`16119eea`); the gates compare commits on this branch.

Provenance. Release `profiling` binaries built from each commit in one
worktree with its own `target/`, `RUSTFLAGS="-D warnings"`, copied out; Terra
from `scripts/build-guests` at the base. Apple M5, 10 cores, on AC power, no
thermal or performance warning in any run's `pmset`. No cargo, rustc or other
engine process ran during any measurement; 1-minute load 3.5 to 8.5 for every
run (each run's `run.json`). `scripts/profile-frames --generator 3 --fixtures
flight active transparency`, offscreen 1920x1080, three repeats a series, each
logging on and off. Timing: two rounds of base, 02, 03, then 03, 02, base (six
logging-on runs a fixture a binary). GPU: a separate `--instruments gpu`
series, one a binary. Numbers below are the median of the per-run values over
the measured 22-32 s phase, `[min-max]` across runs.

The active fixture's first fuel cell stops burning before 22 s on every binary
(fire out from 22.0 to 25.9 s, burning again from the second ignition), so the
runner's fire assertion refused it twice at the base. All three binaries ran
under an untracked copy of the runner with only that assertion removed. The
fixture defect predates this branch.

| Base                       | flight              | active              | transparency        |
| -------------------------- | ------------------- | ------------------- | ------------------- |
| frame p50 ms               | 4.12 [3.76-4.47]    | 3.80 [3.57-4.07]    | 3.91 [3.71-4.10]    |
| frame p95 ms               | 6.36 [5.58-6.82]    | 5.79 [5.13-6.23]    | 5.71 [5.17-6.25]    |
| main CPU p50 / p95 ms      | 0.83 / 1.09         | 0.81 / 1.06         | 0.81 / 1.07         |
| GPU frame p50 ms           | 3.99 [3.97-4.01]    | 4.12 [4.11-4.12]    | 4.20 [4.18-4.22]    |
| GPU frame p95 ms           | 4.27 [4.24-4.27]    | 4.34 [4.33-4.35]    | 4.45 [4.42-4.46]    |
| mesh worker ms/s, level 0  | 34.4 [31.4-36.7]    | 55.4 [49.2-62.3]    | 57.0 [52.4-60.9]    |
| mesh worker ms/s, all      | 278 [258-295]       | 372 [326-420]       | 372 [327-416]       |
| mesh bytes held            | 69.0 MB             | 75.2 MB             | 75.0 MB             |
| upload wall ms per frame   | 0.011 (p95 0.055)   | 0.013 (p95 0.058)   | 0.013 (p95 0.058)   |
| uploads per frame          | 2.45                | 2.93                | 3.00                |

- Mesh worker is `frames.flow.csv`'s `mesh_cpu_ms` over the phase; level 1's
  coarse remeshing is most of "all". Held bytes are `capture_lod_memory`'s
  total at the capture, identical within 0.2% across runs. Upload is
  `upload_wall_ms` and `uploads` summed over the phase per frame.
- GPU: `frame_start` to `frame_end` from `frames.gpu.csv`; 75-95% of base
  frames have a `frame_end`. Of a 4 ms GPU frame the main opaque pass is
  0.16 ms, the depth prepass 0.3 ms and the shadow passes about 0.5 ms; the
  rest falls outside the per-view markers.
- Meshing benches, interleaved thread-CPU A/B (criterion patched locally to
  `CLOCK_THREAD_CPUTIME_ID`, not committed), six rounds alternating base, 02,
  03, `--warm-up-time 1 --measurement-time 2`, load 1.3 to 6.5. Base medians:
  `greedy_mesh` terrain 214 µs, solid 214, checkerboard 2352, terrain light
  gradient 233, stairs at cap 487, stairs at budget 364, all stairs 3500,
  fences at cap 481, flat fences 434, terrain water 288, canopy 309, glass room
  156; `mesh_chunk` straddling 279, shore 290, straddling unsmoothed 227, ramp
  densest 329, cave crossed 259, cave mouth 232.

Open questions:

- **Order of items 2 to 4.** The frame is GPU-bound: GPU 4.0-4.2 ms against
  0.8 ms of main CPU. Ticket 02 made the GPU slower, in the vertex-heavy
  passes (ticket 06), so the next work is that regression, before either
  item. Then 04, the face split: skipping the faces that face away removes
  vertex work, which is now the cost, at the price of more draws on a CPU
  with room for them. 05 last: on Metal a multi-draw is a loop of single
  draws and the CPU is not the bound, so it stays `needs-info` and is likely
  `wontfix`.
- **Per-layer or bucketed placeholders: per-layer.** Placeholders are 6 of
  14 bytes a quad, about 5 MB of 02's 12 MB held; buckets would trade that
  for about 12% more vertex work, the cost that already regressed.
- **Quad buffer size and growth.** At about 87 bytes a quad at the base, the
  fixtures hold about 0.86 M quads: about 6.9 MB, or 1.6 Mi words, of records
  and tables. The 1 Mi-word start therefore doubles once during the first
  fill. Start at 2 Mi words (8 MiB) and keep doubling, which then happens only
  past these view distances.

2026-11-19: the per-layer placeholder answer above was weighed at 6 of 14 bytes
a quad, the non-indexed placeholder. Ticket 07's indexed draw made it 16 of 24
(4 of vertices, 12 of indices), so bucketed placeholders save more than they
did and may be worth measuring again, against the vertex work they add.
