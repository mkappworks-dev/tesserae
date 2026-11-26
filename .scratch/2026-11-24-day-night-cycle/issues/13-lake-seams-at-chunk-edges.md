# 13: Seams on lakes at chunk edges

Status: ready-for-agent

Spec: [Day and night cycle](../spec.md).

## What to build

Floating above a lake and looking a little ahead, the water shows straight box
outlines and a thin green line along chunk edges, like a scan: a coarser level's
water box and the shallow bed under it show through the finer level's water.
Also seen on ground. Find the cause and fix it.

Not caused by this branch's render changes: the same capture with 45c1e246's
terrain fog reverted is identical. It most likely predates the branch (LOD and
water meshing are untouched here), but `main` cannot open the user's World to
confirm, because it rejects the new `daylight_cycle` field.

Reproduce: the user's World "New World" (seed 13925834207698264142, Terra,
Creative) in `~/Library/Application Support/tesserae/worlds/New World`. Copy it
first, never open the original. A `dev` release build with
`TESSERAE_WORLD_DIR=<copy>`, `TESSERAE_SPAWN_M=-3659,100,-1399`,
`TESSERAE_SCREENSHOT_PITCH_DEG=-25`, `TESSERAE_SCREENSHOT_YAW_DEG=0`,
`TESSERAE_SCREENSHOT_SETTLE_S=30`, `TESSERAE_TIME_OF_DAY=12:00`. A fresh World
from the same seed generates different ground (Jungle, no lake), so the copy is
needed. `TESSERAE_SPAWN_M` needs the `dev` feature; a `profiling` build ignores
it.

## Acceptance

- [x] The cause found and written here.
- [ ] No seams on the lake at the spot above after the fix, captured.
- [x] A test that fails before the fix, if the cause is in meshing or LOD
      selection.

2026-11-27, cause: the lake is a Generator version 3 lake at 38.25 m, above
the 24 m sea. `level_the_water` (`server/lod.rs`) moves a coarse water surface
to level 0's height, but only for the generator's one water level, the sea, so
a lake kept its coarse rounding: level 0 draws it at 39 m, level 1 at 40 m. At
the join the level-1 water stood a metre out of the level-0 water as a box, and
the green line was its grass bed showing at the edge. A `TESSERAE_LOD_TINT`
capture puts every outline on the level 0 to level 1 border.

Fix: a mesh job asks its generator for each coarse column's own water level
(`MeshGenerator::water_level_m`, the sea or a lake or river), and the surface
row's sides now end at the levelled surface too, or the seam grid's water
sides stand a metre out of it once the top is lowered. Tests:
`a_lake_surface_above_the_sea_is_drawn_at_level_zeros_height` fails before
(top at 46 m, not 45 m) and passes after; the sea test now also holds the
sides to the surface.

Capture at the ticket's spot, 30 s settle: the boxes and the green lines are
gone. Faint darker bands remain in the water past level 0's edge, on 32 m seam
planes (x -3584 and -3552, z -1600 and -1568). They are not level-1 water
sides or bed seam walls (a Terra probe of those Chunks finds none there), and
a 90 s settle shows the same; not yet explained. Mesh time per coarse job
moved by +5 to 15% in one capture each way, within run-to-run noise.
