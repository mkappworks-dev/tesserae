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

- [ ] The cause found and written here.
- [ ] No seams on the lake at the spot above after the fix, captured.
- [ ] A test that fails before the fix, if the cause is in meshing or LOD
      selection.
