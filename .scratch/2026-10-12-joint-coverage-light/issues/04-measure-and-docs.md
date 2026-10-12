# Measure, and close the loose end in the docs

Status: done
Blocked by: 03

- Before/after table for the spec's Performance list, in the PR and in this
  file's Comments. If `SHAPED = false` moved, rework before anything else.
- DESIGN.md's shaped-voxels loose end loses the joint-coverage sentence.
- Decision 5 of the step-5 spec: the "known limitation" paragraph and its loose
  end become a pointer to this spec.
- `./scripts/cargo-ci` passes.

## Comments

**How it was measured.** The host was under heavy load from other sessions
(load average 50-110), and a wall-clock `--baseline before` run came out 2-8x
slower on every bench, including `light_chunk/opaque`, which this change does not
touch. So the numbers below are from a different harness: the `main` and branch
bench binaries run **interleaved**, ten rounds each, criterion on **thread CPU
time** (`clock_gettime(CLOCK_THREAD_CPUTIME_ID)`), compared on the minimum and the
median per bench. Load average was 8-14 by then. The patch to `meshing.rs` is
not committed.

| `light_chunk/`      | main min | branch min |  Δ min | main med | branch med |  Δ med |
| ------------------- | -------: | ---------: | -----: | -------: | ---------: | -----: |
| straddling          |  93.8 us |    95.1 us |  +1.4% | 100.4 us |    97.8 us |  -2.5% |
| straddling_settled  | 144.1 us |   149.2 us |  +3.6% | 165.4 us |   151.7 us |  -8.3% |
| cave_dark_neighbours|  46.9 us |    45.8 us |  -2.4% |  49.3 us |    47.4 us |  -3.8% |
| cave_mouth          | 110.0 us |   108.1 us |  -1.7% | 121.9 us |   114.6 us |  -5.9% |
| flooded_air         | 127.6 us |   125.5 us |  -1.6% | 146.9 us |   129.9 us | -11.6% |
| open_sky            |   4 ns   |    5 ns    |  +2.6% |   5 ns   |    5 ns    |  +5.2% |
| opaque              |   3 ns   |    3 ns    |  +6.0% |   4 ns   |    4 ns    |  -0.7% |
| **straddling_shaped** | 144.6 us | 181.4 us | **+25.4%** | 152.8 us | 187.8 us | **+22.9%** |

Every bench that runs `SHAPED = false` is within the noise of the run, both ways.
`straddling_shaped` (a stair on every column, the worst realistic case: nearly
every lit cell is beside another partial cell) pays about 37 us for the joint
test and the extra per-cell sweep write. That is within the spec's allowance
("may rise, and the PR says by how much") and still 0.18 ms against a 15.6 ms
tick. An earlier two-sample run at higher load gave the same +26% / +18% for it.

The mutation checks (ten, each killed by a named test) are in the PR description.
