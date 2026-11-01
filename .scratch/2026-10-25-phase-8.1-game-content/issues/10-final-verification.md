# 10: Final verification and the step 8.1 record

Status: done
Blocked by: 07, 09

Spec: [Testing Decisions](../spec.md#testing-decisions).

## What to build

Prove the phase end to end and record it. Run the demo by hand at 1.0, 0.5 and
0.25 m:
- place water, lava and mud from the menu;
- ignite `demo:wood` and put it out;
- build with `demo:brick` in a Mod Shape;
- watch the scripted scenarios.

Confirm that a World with no Game is dry. Add a step 8.1 entry to DESIGN.md's
build order. Remove "the player's fluid tool places water only" from its open
items. Record any limit found as an open item rather than fixing it in passing.

## Acceptance criteria

- [ ] Every ticket 01 to 09 is `Status: done` and its criteria are ticked.
      01 to 06 and 08 are; 07 and 09 are `ready-for-human`, waiting only on
      the user's review of their screenshots.
- [ ] The manual pass above is done at all three Voxel sizes, and its
      screenshots are reviewed with the user and then deleted. The scripted
      part is captured in `/tmp/tesserae-p81-t10/`; placing by hand is left
      to the user. See the Comments.
- [x] Interleaved CPU-time A/B against main shows the meshing, LOD and fluids
      benches within noise.
      It also stands in for ticket 01's own A/B, whose interim run left
      `greedy_mesh/terrain_water` +7.5% unexplained: bench water at id 8 and
      at the built-in 7 in one binary to tell layout from cost.
- [x] DESIGN.md has the step 8.1 entry and updated open items. Data-only Mods
      are listed as the agreed next session.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass on the
      final commit.
- [x] No push, PR or merge without the user's go-ahead.

## Comments

### 2026-10-26: verification

**Fire timing (ticket 09's "one state behind at 12 s").** Not the gallery;
the demo's lakes. Four offscreen captures of the burning row at 1 m and 12 s
(yaw 90, pitch -45, no flight), on this branch with a temporary,
uncommitted switch that skips the gallery and with
`TESSERAE_WATER_LEVEL_M=off`: with the gallery and without it the row is the
same, still burning; with no lakes, with or without the gallery, it is a
step further on, and those two are identical pixel for pixel. A temporary
log of the Mod drain put the demo's two Ignite Commands the same time
before the shutter, within 10 ms, in every run with and without either, so
the scenario starts on time. With lakes the row shows at 13 s what it shows
without them at 12 s, and the two agree at 40 s. Main's demo had no lakes
(`water: None`); ticket 04 gave it lakes at 25.5 m. The likely mechanism,
read from the code and not instrumented: `admit` rescans every arriving
Chunk that holds generated lake fluid once, under the shared Agenda quota,
so at startup the first fuel evaluations wait behind the lake shores. No
rule changes; recorded in DESIGN's open items, not fixed. Evidence in
`/tmp/tesserae-p81-t10/fire-timing/` (`sheet-f12.png`: with lakes top,
without below; with gallery left, without right).

**Bench A/B.** Main (9992e8e) against this branch (c344dc2), the meshing,
collide, lod and fluids binaries of `tesserae-core` built with the
thread-CPU criterion patch (uncommitted, in a temporary detached worktree,
since removed), `--warm-up-time 1 --measurement-time 2`, six interleaved
rounds alternating which side goes first, under `caffeinate -i`. One-minute
load 1.3 to 5.3 throughout, so no round is discarded. 115 rows; these are
the ones beyond 3% either way on the minimum or the median of six:

| bench                              | main min | post min |    min |    med |
| ---------------------------------- | -------: | -------: | -----: | -----: |
| `greedy_mesh/terrain_water`        | 282.8 us | 302.1 us |  +6.8% |  +7.2% |
| `greedy_mesh/solid`                | 222.4 us | 209.8 us |  -5.7% |  -5.3% |
| `greedy_mesh/terrain_canopy`       | 305.5 us | 315.2 us |  +3.1% |  +3.6% |
| `mesh_chunk/shore`                 | 287.1 us | 296.7 us |  +3.3% |  +3.4% |
| `mesh_chunk/cave_mouth`            | 246.5 us | 240.5 us |  -2.4% |  -3.6% |
| `lod_downsample/changed_dry`       | 695.8 us | 710.3 us |  +2.1% |  +4.4% |
| `lod_downsample/changed_flooded`   |   1.2 ms |   1.2 ms |  +3.0% |  +3.2% |
| `lod_mesh_coarse/surface_level_1`  | 261.8 us | 258.3 us |  -1.3% |  -4.2% |
| `lod_mesh_coarse_lit/mouth_…`      | 371.6 us | 363.8 us |  -2.1% |  -3.3% |
| `light_chunk/cave_dark_neighbours` |  41.1 us |  40.9 us |  -0.5% |  -3.6% |
| `fluids_saturated/forget_one_…`    | 134.9 us | 129.9 us |  -3.7% |  -2.7% |
| `snapshot/clone_seven_…`           |   4.8 us |   4.6 us |  -4.6% |  -6.2% |
| `snapshot/arc_clone_seven_…`       |    35 ns |    31 ns | -11.4% |  -0.6% |
| `snapshot/make_mut_shared`         |   320 ns |   292 ns |  -8.8% |  -4.3% |
| `snapshot/make_mut_shared_light`   |   458 ns |   494 ns |  +7.9% | +12.8% |

Every other row is within 3%, every fluids row but one within 2.6%. The
mesher (`mesh.rs`) and `MaterialTable::def` are unchanged from main, yet
mesher rows move both ways by up to 7%, which is a relinked binary; the
`snapshot` rows are nanosecond clones and allocations in code this phase did
not touch. The LOD downsample rows, +2 to 4%, sit in code ticket 02 changed
(the waterline names its fluid) and at the edge of the noise ticket 01 saw
(±4%); not acted on. Conclusion: within noise, with one layout outlier.

**`greedy_mesh/terrain_water`, explained.** A probe built at 2fce99f (before
ticket 09, so built-in water still exists) benched the same terrain under
water at id 7 and at the fixture's id 8 in one binary, three rounds at load
1.8 to 2.2: id 7 with the fixture's Tables 283.1 to 284.0 us, id 8 283.0 to
283.4 us, id 7 with the built-in Tables only 282.5 to 284.4 us, and a
second pass of id 8 then id 7 283.0 to 285.8 and 282.7 to 283.9 us; both
meshes come out the same size. That binary's own
`terrain_water` (id 8) was 283.2 to 283.7 us, main's figure. So the id costs
nothing; the +7% belongs to how each binary lays out the mesher, as the
opposite -6% on `greedy_mesh/solid` from the same unchanged code shows.
Raw data, not committed: `<session scratchpad>/ab/` (`runs.log`,
`crit/`, `summary-r1-6.md`, `probe-out/`); ticket 01's interim run is under
`ab/t01/`.

**Manual pass.** The dev capture cannot click, so what it can show was
captured at 1.0, 0.5 and 0.25 m, offscreen, in `/tmp/tesserae-p81-t10/`
(logs beside them in `logs/`):
- `<size>-demo-scenarios-12s` and `-40s`: the whole stage from above, 40 m
  up. Lava over the wall, mud fired to brick and water cooling lava on the
  beam, the waterfall and its source taken away, the burning row and the
  doused post. At 0.25 m the stage is still being built at 12 s (the Mod's
  Voxel cap, step 7) and runs by 40 s.
- `<size>-demo-lakes`: `demo:water` lakes in the valleys, clear of the
  stage, and the gallery pool in the same fluid.
- `<size>-demo-menu`: the open menu, listing `demo:water`, `demo:lava`,
  `demo:mud`, fire and the demo's solids and Shapes, and no `core:water`.
- `<size>-nogame-dry` and `-nogame-menu`, with `TESSERAE_MODS_DIR` an empty
  directory: no lakes anywhere and no gallery pool; the menu has the six
  built-in solids and five Shapes, no fluid or fire section, and slots 8 and
  9 empty. At 0.25 m the fill (caves on, 192 m) took 42 s, so that one was
  taken later.

Left for the user, by hand, at each size: place water, lava and mud from the
menu; ignite `demo:wood` and put it out; build with `demo:brick` in a Mod
Shape (e.g. `demo:fence`). Then review these screenshots, ticket 07's and
ticket 09's, and delete the three folders.

**Docs.** DESIGN has the step 8.1 entry in step 8 of the build order. Open
items: Data-only Mods as the agreed next session; the trackpad's 100 px notch
untested on hardware; the lakes' startup delay to the first fire. "The
player's fluid tool places water only" was already removed by ticket 06.

`cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass: tesserae 370
(2 ignored), core 326, meshing 69, tesserae-mods 95 (3 ignored).
