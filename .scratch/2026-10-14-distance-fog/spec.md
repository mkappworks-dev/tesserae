# Distance fog

Branch `feat/distance-fog`, on `main` at the merged LOD step (#13). Not a build
step of its own: the LOD spec left "No distance fog or atmosphere" as a known
loose end and said fog was step 9's work. Step 9 still owns the final look. This
lands the default, as plain data, so that step has something to override.

## Problem

The LOD step draws terrain out to 4 km against a flat clear colour. Three things
show:

- A hard horizon: terrain ends at `lod_distance_m` plus a Cell, where the far
  cameras' `far` plane cuts it, and the sky behind it is a different colour.
- Every ring join is a step of one coarse Voxel, between a surface and the one
  beside it, at full contrast whatever the distance.
- The reach looks like a limit of the engine and not a choice.

Fog replaces a growing share of a surface's colour with one colour as it gets
further from the Eye. It lowers contrast with distance, which softens the joins,
and ends the terrain in the sky colour, which makes the horizon the sky's.

## Vocabulary this step adds

One term, to be added to `CONTEXT.md` in the same PR.

- **Distance fog** - how much of a surface's colour is replaced by the fog
  colour, as a function of its distance from the Eye in metres. Sized from the
  full-detail radius and the reach, so it is the same fog at any Voxel size.

## What the design rests on

Read from the sources under `~/.cargo/registry/src/*/bevy_pbr-0.20.0`, not from
memory, because the decisions below only make sense once these are known.

- **Fog is a component on a camera.** `DistanceFog` (`src/fog.rs`) is extracted
  per view and written to one uniform per view (`src/render/fog.rs`,
  `prepare_fog`). A view that has one gets `MeshPipelineKey::DISTANCE_FOG`
  (`src/render/mesh.rs`), which defines `DISTANCE_FOG` in the material's shader
  and adds the fog binding to the view layout. So each of the LOD cameras needs
  its own, and a camera without one draws unfogged.
- **The chunk shader already ends in the fog call.** `apply_fog` runs inside
  `main_pass_post_lighting_processing` (`src/render/pbr_functions.wesl`), which
  `chunk_material.wesl` calls last, gated by the material's fog flag.
  `StandardMaterial::fog_enabled` defaults to true and the chunk material's base
  is a default `StandardMaterial`. So fog needs **no change to the shader**, and
  the sessions that edit it (lighting the coarse levels) have nothing to
  conflict with.
- **The distance is Euclidean from the camera's world position, per fragment.**
  `length(fragment_world_position - view.world_position)`. No depth buffer, no
  view-space z. Every LOD camera is a child of the near camera at an identity
  transform, so `view.world_position` is the same in all of them, and fog is a
  function of the fragment's world position alone. The same fragment is fogged
  identically whichever camera draws it: there is no step at a camera boundary
  by construction, and what stays to check is that the five parameter sets are
  equal.
- **Fog is applied before the colour is tonemapped, and the clear colour is
  not.** The cameras are not HDR, so tonemapping runs in the shader after the
  fog (`TONEMAP_IN_SHADER`); the clear colour is written to the target as it is.
  So a fully fogged fragment is the tonemapped fog colour and the sky is the
  fog colour untouched, and equal values do not make equal pixels. See
  decision 3.
- **The fog colour is a `Color`**, converted with `LinearRgba::from`, so an sRGB
  value is converted correctly. `ClearColor` is a `Color` too.
- **The falloff modes** are `Linear { start, end }`, `Exponential { density }`,
  `ExponentialSquared { density }` and `Atmospheric`. The three non-linear
  densities are per metre because the fragment distance is in world units, and
  the world is in metres.
- **Fog touches the output colour only.** It does not discard, write depth or
  change alpha, so the cut and the debug tint in `chunk_material.wesl`, which
  run before lighting, are untouched: the cut `discard`s before fog is reached,
  and the tint is applied to `base_color` before it.

## Decisions

### 1. One `DistanceFog` on every camera, from one function

`render/fog.rs` holds the data and the one function that turns it into Bevy's
component. `spawn_camera` in `client/chunk_view.rs` attaches the result to the
near camera and to every coarse camera. That is a handful of lines in a file
other sessions edit (the blended draw call changes `upload_meshes`), and no line
of the shader.

The alternatives, and why not:

- _Fog in the chunk shader._ It would be one more uniform and one more thing the
  coarse-light work edits in the same file, to reimplement what `apply_fog`
  does. It is also the version that would not fog a blended surface, which the
  other session's material will be a `StandardMaterial` for.
- _A post-process pass._ It needs the depth texture of five cameras that each
  clear their own, and it fogs the sky.

A blended surface that goes through `apply_fog`, as a `StandardMaterial` does,
is fogged by the same camera component with no further work. That is the
interaction to tell the blended-pass session about.

### 2. The numbers: plain `Copy` data, in metres, derived

```rust
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct FogSettings {
    pub color: [f32; 3],
    pub reach_opacity: f32,
}
```

`color` is sRGB, the space of `ClearColor`. `reach_opacity` is the share of a
surface's colour that fog replaces at the reach, 0 to 1. Both have a `Default`
and neither is a Voxel quantity, so a Look pack in step 9 overrides them as it
would any two numbers, and nothing crosses the WASM boundary per Voxel
(ADR-0001).

**The reach** is `max(lod_distance_m, view_distance_m)`: how far the farthest
drawn terrain is, which is `lod_distance_m` unless the World has no coarse
levels. **The density is derived from it**, so the opacity at the reach is
`reach_opacity` whatever the reach is:

```
density = sqrt(-ln(1 - reach_opacity)) / reach_m          (exponential squared)
```

Distances are metres end to end (ADR-0003). No Voxel size appears, and the same
`FogSettings` over a World with 0.25 m Voxels gives the same fog; a test pins
that by constructing the fog from the two distances alone.

**The falloff is exponential squared**, for the property the other two lack: `1
- exp(-(d D)^2)` starts slowly. At the defaults (reach 4096 m, 0.85) it is 0.4%
at the edge of level 0 (192 m), 1.7% at the join of levels 1 and 2 (384 m), 6%
at 768 m and 66% at 3 km, so the full-detail rings are untouched and the fog
works on the coarse ones. Plain `Exponential` is 8.5% at 192 m, which is haze on
the ground the Player is standing on; `Linear` has a kink where it starts.
Ticket 03 renders the first two on the same views before it keeps this, and
records the choice. Which curve is an implementation of the same two numbers, so
the data does not change if it changes.

**Defaults.** `color` is the clear colour's own sRGB value, today's
`ClearColor` default (43, 44, 47). `reach_opacity` starts at 0.85: at the reach
terrain keeps 15% of its colour, which is the "visible but faint" the scope
asks for. Both are tuned against captures in ticket 03 and the final numbers are
recorded in the Outcome.

### 3. The fog colour is the clear colour, set from one value

The Client inserts `ClearColor` from `FogSettings::color`, in the plugin and so
before any other system. The dev-only `TESSERAE_SCREENSHOT_MAGENTA_SKY` probe
replaces `ClearColor` after that, in `capture.rs`, so it keeps working: the
fog colour is untouched, the sky is magenta, and a fogged fragment is still not
magenta, which is what the probe counts.

That fixes the _values_. The _pixels_ differ by the tonemap (above), and by how
much is measured and not assumed. Ticket 02 captures a view with the fog so
dense that the terrain is all fog colour and compares it with the sky pixels. If
they agree within 3/255 per channel the single value stands. If they do not, the
clear colour is set to what the tonemapper makes of the fog colour, as a named
constant with a one-line comment that says why, and a test pins the relation to
the constant. The look is what is matched, and it has to be matched by looking.

The sky is not otherwise touched: no gradient, no atmosphere, no clouds.

### 4. The same fog in the near layer and the far layers

All of the cameras are given the _same_ `DistanceFog`, built once and cloned. A
test queries every camera that draws terrain and asserts all six components are
equal, field by field (`DistanceFog` is not `PartialEq`). Together with "the
distance is from the camera's world position" this is the whole argument for no
step at the camera boundary, and a capture checks it: a row of pixels across
the level 0 to 1 join, where the near camera's last fragment sits next to the
coarse camera's first, shows no change in the fog gradient.

### 5. Dev switch

`TESSERAE_FOG=0` under `dev` attaches no `DistanceFog` at all, in the way
`TESSERAE_LOD=0` removes the coarse levels. A zero-opacity fog would still cost
the fog binding and the branch, so it cannot stand for "off" in a measurement.
The same switch makes a before capture from the same binary. A build without
`dev` has no switch and always fogs.

### 6. What a Mod or a Look pack does

Nothing per Voxel. `FogSettings` is two numbers that step 9's Look-pack API sets
once at load, in the post-process settings DESIGN.md already lists ("fog
parameters"). The derivation from `view_distance_m` and `lod_distance_m` stays
in the engine, so a pack gives a colour and an opacity and not a density that is
right for one reach only. Re-deriving `ClearColor` when the settings change is
step 9's, since nothing changes them at run time yet.

### 7. Everything physical stays in metres

The distances are `view_distance_m` and `lod_distance_m`, the density is per
metre and the fragment distance is in metres. There is no Voxel count in the
code, and the only dimensionless numbers are the opacity and the colour.

## Invariants, and where each is enforced

1. **The Client never mutates World state.** Fog is render state on cameras. It
   reads `StreamingSettings` as `spawn_camera` already does and writes nothing
   to the Server.
2. **No Mod reaches `tesserae-core`.** No host call is added, and
   `tesserae-core` is not touched.
3. **`tesserae-core` gains no Bevy or wgpu.** Nothing is added to it.
4. **Everything physical is in metres.** Decision 7, and a test builds the fog
   from the two distances without a Voxel size.
5. **The depth cut and the debug tint are unchanged.** `chunk_material.wesl` is
   not in the diff, and a capture with the tint on and one near the ground are
   looked at for a coarse bump through the fine layer.

## Performance

Fog is on the GPU: one extra uniform binding, and per fragment a distance, an
`exp` and a `mix`. It adds no main-thread work and no Server work, so
`MainThreadCost` and the worst tick are expected not to move, and are reported.

- **Frame cost** is measured with `scripts/frame-runs` against a build of `main`
  in its own worktree, interleaved, minimum or median of several runs, after
  `uptime` is read. Wall-clock figures lie above a load of about 5 and the 60 Hz
  cap hides a fast frame, as the LOD outcome found; a run at the cap is left out
  and said so. `TESSERAE_FOG=0` on the branch is the second baseline, the one
  that isolates fog from everything else on the branch.
- **The pipeline.** `DISTANCE_FOG` is a specialisation key, so the chunk
  pipeline is built once more per view layout. That is a load-time cost, once,
  and is noted if it shows.

## Known loose ends, declared rather than discovered

- **No sky, atmosphere, sun or clouds.** The sky stays a flat colour, and a
  fully fogged horizon is exactly that colour. Out of scope by the request.
- **No Look-pack API.** The numbers are data with a `Default`; setting them is
  step 9's.
- **`ClearColor` is not re-derived at run time** when `FogSettings` changes.
  Nothing changes it yet.
- **Fog is uniform in direction.** No height fog and no directional glow
  (`directional_light_color` is `NONE`), so there is no sun glow around the sun.
- **A blended surface is fogged only if its shader goes through `apply_fog`.** A
  `StandardMaterial` does. The blended-pass session decides its material, and is
  told.
- **The far plane still clips at `lod_distance_m` plus a Cell.** At 85% fog the
  clip edge is a step of the remaining 15%, which is the "faint" the reach is
  allowed to show. A higher `reach_opacity` hides it and costs the visible
  4 km; ticket 03 weighs the two.

## Coordination

Four other branches exist. `git worktree list` and their diffs at the start
of this work:

- `tech/lod-coarse-light-ticket` (lighting the coarse levels): a ticket only. It
  will touch `server/lod.rs` and possibly `chunk_material.wesl`. This branch
  touches neither.
- `claude/friendly-agnesi-4851c8` (blended draw call for water and glass):
  uncommitted tickets under `.scratch/2026-10-12-material-table-transparency/`.
  It will change `upload_meshes` in `client/chunk_view.rs`, which this branch
  does not, and it lists distance fog as out of scope in its ticket 06. This
  branch edits `spawn_camera` in the same file, a different function.
- `claude/brave-babbage-b12e1c` and `origin/tech/coarse-lod-durable-edits`
  (coarse LOD edits and emitters): a ticket only, server side.
- The tree is rebased on `main` before the PR.

## Done when

`cargo dev` shows fog on terrain from near to 4 km, with captures taken, looked
at and described: the horizon at the default vantage and at 150 m, near the
ground, with the debug tint, and the join between the near and the far layers.
The fog colour and the sky agree as measured. `./scripts/cargo-ci` passes. The
frame-cost comparison against `main` is in the PR. `DESIGN.md`,
`docs/architecture.md` and `CONTEXT.md` are updated. The screenshots are in
`/tmp/tesserae-fog-review/`, were never staged, and the folder is deleted. If
part of the scope is blocked, the rest is finished and the PR says what was
left out and why.

## Outcome

Written as the tickets finish; what changed from the text above, and why.

### Ticket 03: tuning against captures

Apple M5, `TESSERAE_SCREENSHOT_OFFSCREEN=1`, 1080p, `dev` build; the images are
in `/tmp/tesserae-fog-review/` and not committed. Every capture below that
wants the whole reach in sets `TESSERAE_SCREENSHOT_FLY_AFTER_S=30`.

**Why the first captures looked unfogged.** Three reasons, all measured.

- _The captures were early._ The flight starts when the Server reports settled
  (3.8 s), and levels 4 and 5 fill much later (11 to 14 s and more on a loaded
  machine). The first horizon capture logged `held/meshed` of 2038/2167 for
  level 4 and 0/0 for level 5, so its reach was 3.6 km and not 4.1 to 5.2 km.
  With `FLY_AFTER_S=30` all six levels are in (level 5: 1243 held, 328
  meshed, 4096 to 5221 m).
- _Perspective squeezes the far field into a few rows._ The Eye is 14 to 20 m
  above the terrain at the default vantage. A linear fog from D to D + 1 m
  turns everything past D into sky, so the rows that turn sky-coloured say how
  many rows lie past D. Terrain fills rows 545 to 1080. Past 300 m it
  lies above row 627, past 600 m above row 585, past 1500 m above row 551 (11
  rows), past 3000 m above row 547. Row 545 is 100% past 1.5 km and 43% past 3
  km. So 85% fog, which is only reached at 4 km and out, covers two or three
  rows of 535. The curve was never going to show on a view from near the
  ground; a 150 m view has about six times the rows (the same far field spans
  about 25 rows at 150 m and 4 at 14 m).
- _Fog is mixed before the tonemapper, and what survives reads bright._ With the
  share forced the same everywhere (a linear fog from -p * 1e6 to (1 - p) *
  1e6 m) the dark default sky (43, 44, 47) gives, for typical terrain
  (117, 166, 96) at row 800: p 0.5 (94, 135, 79), 0.7 (80, 114, 69), 0.85 (66,
  91, 61), 0.9 (61, 80, 57), 0.95 (53, 66, 53), 0.99 (47, 51, 50). The green
  left above the sky is 75%, 57%, 39%, 30%, 18% and 6% of the unfogged
  excess. At the brightest horizon row, (152, 206, 127), p 0.85 is (85, 123,
  73) and 0.95 is (62, 84, 59). So 85% of the radiance replaced still leaves
  about 40% of the displayed contrast, and the default row 545, (111, 161, 92),
  is the picture of p about 0.65. I did not read the scene radiance, so "the
  tonemapper compresses it" is the reading that fits, not a measurement.

**The colour.** The dark sky made distance read as gloom: the far terrain went
darker and greener-black against a near-black sky, not hazy. A flat clear colour
and fog colour of a mid blue-grey, (126, 146, 168), reads as air at 150 m; a
paler one, (172, 192, 210), reads as air too but the HUD text is near white, and
white text on it is 1.9 to 1, against 3.2 to 1 on the mid colour (the dark one
is about 14 to 1). The mid colour is kept. **This changes the sky's flat colour
and the fog colour, and nothing else:** no gradient, no sun disc, no atmosphere.
The stated scope did not literally include the clear colour.

Making fog colour and sky one value no longer works for a light colour, and
this is the surprise of the ticket. The tonemapper darkens a bright fully
fogged fragment but the clear colour is not tonemapped. Fully fogged terrain
against the sky, 8-bit sRGB, medians of terrain rows 600 to 930: dark (45, 46,
49) against (43, 44, 47), +2; mid colour as both (119, 135, 154) against (126,
146, 168), -7, -11, -14; pale (151, 165, 178) against (172, 192, 210), -21, -27,
-32. Only the dark one is within ticket 02's bar of 3/255. So `FogSettings`
has a second field, `sky`, and the fog colour is the one that, once tonemapped, comes
out as the sky: fog (136, 160, 190) gives terrain (126, 145, 169) against the
sky (126, 146, 168), within 1/255, and every fully fogged pixel is within 1
level of its median (p5 and p95 are (125, 144, 168) and (126, 145, 169)). Two
iterations of measure and adjust found it; the relation is a named constant
with a test that pins the pair and that fog is brighter than sky in every
channel. A Look pack that changes `color` must change `sky` with it. Deriving
one from the other needs the tonemapper's LUT on the CPU and is step 9's.

**The curve and the opacity.** Fog share by distance, from the formulas:

| Curve, reach_opacity | 192 m | 384 m | 768 m | 1.5 km | 3 km | 4096 m | 5221 m |
| -------------------- | ----- | ----- | ----- | ------ | ---- | ------ | ------ |
| squared 0.75         | 0.3%  | 1.2%  | 4.8%  | 17.7%  | 54%  | 75%    | 89.5%  |
| squared 0.85         | 0.4%  | 1.7%  | 6.5%  | 23.4%  | 66%  | 85%    | 95%    |
| squared 0.88 (kept)  | 0.5%  | 1.8%  | 7.2%  | 25.8%  | 70%  | 88%    | 97%    |
| squared 0.92         | 0.6%  | 2.2%  | 8.5%  | 29.9%  | 76%  | 92%    | 98%    |
| squared 0.97         | 0.8%  | 3.0%  | 11.6% | 38.9%  | 86%  | 97%    | 99.7%  |
| plain 0.85           | 8.5%  | 16.3% | 29.9% | 50.9%  | 76%  | 85%    | 91%    |
| plain 0.92           | 11.2% | 21.1% | 37.7% | 61.2%  | 85%  | 92%    | 96%    |

Seen at 150 m with the new colours, over the strip of rows around the horizon:
squared 0.85 leaves a soft edge about 20 rows deep with the far terrain still
readable; 0.92 dissolves it over about 35 rows and the terrain past 4 km is
nearly gone; 0.97 dissolves it over about 60 rows and the clip distance is
invisible, at the cost of the 4 km the LOD step built. Plain exponential at
0.85 is the strongest at a low Eye, a veil over everything past 100 m, and
puts 8.5% of fog on the ground at the edge of level 0 and 4.5% at 100 m, which
is the haze on the Player's own ground that the ticket rules out. So the curve
stays exponential squared. 0.75 is nearly unfogged. **Kept: squared, 0.88.** It
is between 0.85 and 0.92 on the look and is the largest opacity that keeps the
join of levels 1 and 2 under 2% (1.8%; 0.9 is 2.003%), which ticket 01 pinned
as the spec's figure, so no test of ticket 01 changed except that the literal
0.85 in the no-coarse-levels test reads the default. Level 0's edge is 0.5%.

At the reach, terrain at 4.1 to 5.2 km (level 5) is visible and faint. From
300 m up, level 5's rows are 479 to 503, and the median pixel over columns 300
to 1700 against the sky (126, 146, 168), 8-bit sRGB: row 479 (127, 149, 167),
rows 481 to 491 (131 to 133, 159 to 165, 164 to 160), rows 499 to 502 (134, 170
to 171, 156 to 155). The unfogged terrain there is (150, 205, 125). So the
difference from the sky is at most 25 in green and 13 in blue, and the very
far edge is within 3 of the sky: a faint green on the haze, no clip edge.

**What fog could break, checked.**

- _The camera boundary._ The fog share is read per pixel from a fogged and an
  unfogged capture of the same Eye (same-pixel ratio of the channel furthest
  from the sky), as the median over columns aligned on the join, taken from a
  tinted capture that says which pixels are which level. At the default density
  the fog is below one 8-bit step at the first joins (0.0 at the join of levels
  0 and 1, 1.9% flat across levels 1 and 2 at 150 m), so a second run with
  throwaway reach opacity of 0.99999 raises the signal: 3.7% flat
  across levels 0 and 1 (rows 984 +- 8), 5.8 to 5.9% across levels 1 and 2, and
  smooth on the way to 90% at levels 3 and 4. No line, and the crops at 2x of
  both joins show none. Levels 2 and 3 are noisy (19 columns, 0.1 to 0.4 swing
  by row, no step larger than the swing) because few columns have a clean run
  of each; I read that as the limit of the measure there, not a step.
- _Mutation._ With the five coarse cameras unfogged (throwaway, reverted), the
  dense run reads 0.037 on the near side of the join of levels 0 and 1 and
  0.000 on the far side, and 0.000 everywhere past it; at the default density
  the capture is indistinguishable from `TESSERAE_FOG=0` (a hard horizon). The
  check sees the failure, at the first join only under the dense probe: the
  design makes the first joins nearly fog-free on purpose.
- _The cut and the tint._ `TESSERAE_LOD_TINT=1` at the near-ground view (Eye
  (0, 64, -57), 7 m short of the corner at z = -64, level 0 under the Eye and
  rings out to the horizon): no coarse block over the level-0 horizon; the
  level-1 hill at the right edge is terrain past level 0's ground. At 150 m the
  rings are green, salmon, yellow, light green, cyan, lilac from near to far,
  all still told apart through the fog. Straight down from 150 m is level 0 only.
  The magenta probe over seven views (Eye on the lattice line at 76 m, 30
  degrees at 76 m, 64 m, 150 m, 300 m, straight down at 60 m and 150 m) counts
  no clear-colour pixel under terrain in any column. A pixel painted into the
  capture is counted, so the counter works.
- _Not measured._ The effective fog share is a ratio of 8-bit display values
  between two captures with a pose difference of up to 0.3 m in height, so it
  is good to a few hundredths and cannot resolve joins at the level of the
  quantisation; frame cost is ticket 04's.

**What did not work.** The first idea for the join check, reading the share
from the tinted fogged and unfogged pair, is wrong: the tint changes the base
colour by ring and the display response differs, so shares in different rings
do not compare. It needs the untinted pair and a tinted one only to find the
rows. And `FogSettings::color` as the one colour for fog and sky, the spec's
decision 3, holds for the dark colour only.

### Ticket 04: frame cost against main

Apple M5, `dev` build, `TESSERAE_SCREENSHOT_OFFSCREEN=1`, 1080p, the default
flight (12 m/s, from when the Server reports settled), `scripts/frame-runs 1`
once per run so the three arms could be interleaved. `main` was built in its own
detached worktree (`f57f72d`, which is `14a8fae` plus one docs-only ticket file,
so the same code the branch forked from) with its own `target/`, and removed
afterwards. Logs are in `/tmp/tesserae-fog-review/ticket04/`, not committed.

Three arms, interleaved main, branch, branch with `TESSERAE_FOG=0`, repeated 14
times (42 runs; the first six rounds were planned and eight more were added
because three of the first six `main` runs sat at the cap). Median of each
figure over the kept runs, with the range in brackets, in ms:

| Arm                 | median_flight_ms | p99_flight_ms | worst_flight_ms | worst_tick_ms | kept | left out |
| ------------------- | ---------------- | ------------- | --------------- | ------------- | ---- | -------- |
| main                | 5.3 (5.1-6.7)    | 9.2 (7.3-17.8)  | 20.7 (8.3-35.8) | 1.3 (0.8-1.9) | 9    | 5        |
| branch              | 5.4 (5.0-6.8)    | 8.3 (7.1-23.3)  | 18.9 (7.7-69.1) | 1.1 (0.8-2.7) | 11   | 3        |
| branch, `FOG=0`     | 5.3 (5.1-6.4)    | 8.3 (7.2-16.8)  | 10.3 (7.8-101.2) | 0.9 (0.7-2.3) | 9    | 5        |

The sorted `median_flight_ms` of the kept runs: main 5.1 5.1 5.1 5.3 5.3 5.4 5.9
6.4 6.7; branch 5.0 5.1 5.1 5.2 5.4 5.4 5.5 5.6 5.8 6.4 6.8; `FOG=0` 5.1 5.1 5.1
5.3 5.3 5.8 6.0 6.2 6.4.

**Left out.** A run is left out when its median flight frame is at the 60 Hz cap
(16.3 to 16.7 ms; `TESSERAE_SCREENSHOT_VSYNC` is unset, yet the OS paced these
runs to the display anyway, and I did not find out why) or is within sight of
it. main: 5 (16.3, 16.6, 16.6, 16.6, 16.7). branch: 3 (16.6, 16.6, and one at
14.8 whose capture ended before level 5 was shown, at a load of 7.4 after it).
`FOG=0`: 5 (15.9, 16.6, 16.6, 16.6, 16.7). That is 13 of 42 runs, and it fell on
all three arms alike, so it does not favour one. A run whose median is under 14
ms is kept even when its p99 or worst frame is bad: those are the figures that
vary, and dropping runs for them would be choosing the answer.

**Load.** `uptime` was read before and after every run. The driver waited for a
1-minute load under 5 before starting each run, and the readings before were 3.9
to 5.0 (main 4.2 to 4.95, branch 4.4 to 4.9, `FOG=0` 3.9 to 5.0). The machine
never got quieter than that: other sessions were building and running all the
while, so "load under about 5" here means just under it, not a quiet machine.
After a run the load was 3.9 to 7.4, and in 28 of the 42 runs it was over 5 (up
to 7.4 after branch run 4, 7.2 after main run 14, 7.1 after `FOG=0` run 14). I
could not get the load below about 4. That is why nearly a third of the runs sat
at the cap, and why I read this as a comparison inside one noisy distribution
and not as absolute frame times.

**Conclusion: the fog cost is not measurable.** The median flight frame is 5.3
ms on `main`, 5.4 ms on the branch with fog and 5.3 ms on the branch without it.
The differences between arms (0.1 ms, 0.0 ms) are a fifteenth or less of the
run-to-run range inside any one arm (5.1 to 6.7, 5.0 to 6.8, 5.1 to 6.4 ms), and
the minimum of the medians, the least load-contaminated figure, is 5.1, 5.0 and
5.1. Taking the 11 and 9 kept runs the two arms overlap almost completely, so
fog is costing less than about 0.3 ms a frame at 1080p on this GPU, and I cannot
say how much less, or whether it is zero. The p99 and worst frames are not
evidence either way: the arm with the largest worst frame is the unfogged one
(101 ms, one run), the p99 medians are 9.2, 8.3 and 8.3, and all three ranges
are wide because of the load. The worst tick is 1.3, 1.1 and 0.9 ms, all inside
a range of 0.7 to 2.7 ms: fog adds no main-thread or Server work, as expected.

**Does it grow with the cameras?** Not measured. The whole six-camera fog is
inside the spread, which bounds the per-camera cost at a sixth of that, and
nothing here can say more. A run with `TESSERAE_LOD=0` (one camera) would
not resolve a cost that small either, so I did not run it.

**The pipeline specialisation.** `DISTANCE_FOG` builds the chunk pipeline once
more per view layout. The time `l0_shown_s` (level 0 had every Chunk of its mesh
shell sent and drawn) is 1.71 s on `main` (1.52 to 2.13), 1.71 s on the branch
(1.60 to 2.30) and 1.61 s without fog (1.53 to 2.47), and `l5_shown_s` is 10.91,
10.81 and 10.75 s. These measure when the Server's streaming had finished, not
the pipeline build alone, and the differences are well inside the spread, so a
once-only build of a few milliseconds would not show; if it moved anything it is
under what this can see.

## Tickets

| #   | Title                                       | Blocked by |
| --- | ------------------------------------------- | ---------- |
| 01  | The fog numbers and their derivation        | -          |
| 02  | Fog on every camera, matched to the sky     | 01         |
| 03  | Tune the fog against captures               | 02         |
| 04  | Frame cost against main                     | 03         |
| 05  | Docs and the PR                             | 03, 04     |
