# 07: Win back item 1's frame time

Status: done
Blocked by: 06

Spec: [Packed quads](../spec.md).

## What to build

Ticket 06 failed 02 on frame time: p50 +8-13% and p95 +13-26%, with the GPU's
depth prepass and shadow passes slower and main CPU p50 +11-42%. Find and fix
both costs while keeping the record contract (ADR-0030), then gate again.

- **GPU.** The placeholder is drawn non-indexed, six vertices a quad, so the
  vertex shader and its quad buffer reads run six times a quad in every pass,
  against four with the old index buffer's vertex reuse. Try drawing it
  indexed: four placeholder vertices a quad and the shared `0 1 2 0 2 3`
  pattern, so the corner is the index modulo 4. Measure the extra index bytes
  against the 84% saved.
- **Main CPU.** Not yet attributed. Profile the item 1 binary's main schedule
  against the base's (quad buffer writes and extraction, `Aabb`s, the blended
  sort, specialization) and fix what shows.
- **Coarse remeshing.** Level 1's mesh worker time rose, so all levels are
  1-10% higher; look at `level_the_water` and `split_quads` on records.

## Acceptance criteria

- [x] What costs main CPU, with the profile that shows it.
- [x] 02 plus this ticket against the base under ticket 06's method: no worse
      frame p50 and p95, held mesh bytes still well below the base. Numbers
      here.
- [ ] ~~If it still fails, the numbers here and item 1 dropped, with item 3
      ported back to the old vertex output.~~

## Comments

2026-11-19: two fixes, `45ece2ac` (CPU) and `ee263f89` (GPU); the branch now
passes the gate against the base.

**Main CPU: what costs it.** Profiled with macOS `sample` (8 s at 22-30 s of
the transparency fixture, one logging-on run per binary) on the base
(`16119eea`) and 02 rebased on it (`ae1e5600`). No system on the main thread
grew: its busy samples were about equal, and the deltas were executor waits
and wakes. The cost was the Client's per-mesh build task, `Built::new`, which
runs on the compute task pool the main schedule's systems share: 30-46 samples
per pool thread at the base (`chunk_mesh_to_mesh`), 100-126 with 02
(`ChunkQuads::pack`), about 80% of it in `Quad::corners`, which pack called on
every quad only to take the layer's `Aabb`. The render thread, which also runs
pool tasks, showed the same.

- `45ece2ac`: `Quad::bounds` reads a face's box from its words (a flat form
  still folds its corners), and pack looks each paint word up once. Tested
  equal to the corners' fold for every kind.
- In this gate main CPU is level with the base. Ticket 06's +11-42% did not
  reproduce at this base: in the profiling runs 02's main CPU p50 was already
  0.64-0.65 ms against the base's 0.69-0.70.

**GPU.** `ee263f89` draws the placeholder indexed: four one-byte vertices a
quad and the 0 1 2 0 2 3 pattern (two-byte indices up to 65,536 vertices, four
past them), the corner being the index modulo 4. The expansion and Chunk view
tests now draw through the placeholder's own indices. The shadow passes, which
02 had made slower, are back to the base's (Player camera, `prepass_end` to
`main_start`, median of three runs, one `--instruments gpu` series a binary):

| GPU, ms                   | flight             | active             | transparency       |
| ------------------------- | ------------------ | ------------------ | ------------------ |
| frame p50, base/02/tip    | 4.10 / 4.57 / 4.04 | 4.39 / 4.82 / 4.36 | 4.46 / 4.93 / 4.40 |
| frame p95, base/02/tip    | 4.47 / 4.98 / 4.42 | 4.64 / 5.21 / 4.70 | 4.70 / 5.34 / 4.78 |
| shadows, base/02/tip      | 0.56 / 0.83 / 0.51 | 0.78 / 1.05 / 0.77 | 0.78 / 1.05 / 0.77 |
| depth prepass, base/02/tip | 0.30 / 0.27 / 0.28 | 0.28 / 0.24 / 0.25 | 0.28 / 0.25 / 0.25 |

- `frame_end` is on 80-87% of base frames, 18-24% of 02's and 19-44% of the
  tip's, as in ticket 06, so the frame rows lean on the frames that kept it.
  The pass spans are on every frame.
- The index buffer costs 12 bytes a quad (two-byte indices) and the vertices 4
  instead of 6: held memory is -70% of the base instead of -84%.

**Coarse remeshing.** Nothing to fix: sampled, the coarse mesh job
(`mesh_coarse`) took 502-514 samples per async thread with 02 against 592-621
at the base, and level 1 and up fall in the gate below (with item 3).

**The gate.** Base `16119eea` against the branch tip `ee263f89` (02, 03 and
this ticket), each a release `profiling` build, `RUSTFLAGS="-D warnings"`, from
this worktree's own `target/`; Terra built by `scripts/build-guests` at the
base (unchanged on the branch). Apple M5, AC power, no thermal warning. No
cargo or rustc ran during any series; 1-minute load 3.4-7.2 (each `run.json`),
under `caffeinate -i`. `scripts/profile-frames --fixtures flight active
transparency`, offscreen 1920x1080, three repeats a series, logging on and
off; two timing rounds, base then tip, then tip then base. The active
fixture's first fuel cell still goes out at 22-26 s on every binary, so all
series ran under an untracked runner copy with only the fire assertion
removed, as in ticket 06. Values are the median over a series' three
logging-on runs of each run's fixture-phase value; changes are against the
base in the same round, as round 1 / round 2.

| Tip vs base                | flight          | active          | transparency    |
| -------------------------- | --------------- | --------------- | --------------- |
| frame p50 ms, r1           | 4.05 → 3.88     | 4.15 → 4.13     | 4.20 → 4.18     |
| frame p50, r1 / r2         | -4.3% / -15.0%  | -0.4% / -4.0%   | -0.6% / -0.8%   |
| frame p95 ms, r1           | 5.63 → 5.48     | 5.36 → 5.06     | 5.37 → 5.04     |
| frame p95, r1 / r2         | -2.7% / -16.7%  | -5.7% / -22.3%  | -6.2% / -20.2%  |
| main CPU p50, r1 / r2      | -0.2% / -14.4%  | -2.8% / -22.2%  | -1.1% / -23.2%  |
| main CPU p95, r1 / r2      | 0% / -19.2%     | -4.6% / -25.7%  | -5.3% / -26.2%  |
| mesh worker ms/s, level 0  | 33.9 → 19.1     | 66.5 → 32.6     | 66.3 → 31.7     |
| mesh worker ms/s, all      | 281 → 207       | 418 → 292       | 403 → 282       |
| mesh bytes held            | 81.5 → 23.9 MB  | 81.8 → 24.9 MB  | 81.6 → 24.8 MB  |
| upload wall ms per frame   | 0.010 → 0.018   | 0.014 → 0.025   | 0.014 → 0.024   |
| uploads per frame          | 2.24 → 2.12     | 3.55 → 3.50     | 3.52 → 3.45     |

- Round 2's base series ran slower than round 1's in every measure (frame,
  main CPU, mesh worker) with the tip's two series level with each other, so
  round 1 is the conservative comparison. It alone passes: no fixture worse in
  frame p50 or p95. Flight's p50 runs all lie below the base's; active's and
  transparency's p50 lie within the base's spread, and their p95 runs all lie
  below it.
- Mesh worker and the rows below it are round 1. Mesh worker includes item
  3's bitmask pass 1, which ticket 06 credited with -25-30% at level 0.
- Upload wall per frame stays about 0.01 ms worse: each landing now also
  uploads the placeholder's index buffer. It is not a gate measure.

**Captures.** Every run's offscreen capture of both rounds and the GPU series,
compared pixel by pixel against the base run with the same label. Base against
base (round 1 against round 2) differs in 0.004-0.95% (flight), 0.008-0.18%
(active) and 0.002-0.09% (transparency).

- Transparency: tip against base 0.005-0.14%, at that level.
- Active: 0.005-0.26%, the water curtain's flowing edge, as in ticket 06.
- Flight: most 0.01-1.4%, distant canopy tops and a strip of far terrain
  landed or not, the same places that differ base against base. Four logging-
  off captures differ in 14-79%: three were taken at another Eye (z -875.87,
  -876.05 and -876.99 against -877.18, the capture frame landing elsewhere on
  the flight), and one is the sky a level lighter in blue across the frame.
- No packing error seen. Captures and diff images are in a `/tmp` review
  folder, not committed.
