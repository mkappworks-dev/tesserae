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
  full-detail radius and the reach, so it is the same fog at any Voxel size. The
  fog colour and the colour of the sky behind the terrain are two values (see
  decision 3).

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
    pub sky: [f32; 3],
    pub reach_opacity: f32,
}
```

`color` is sRGB, what the shader fogs toward. `sky` is sRGB too, the clear
colour: a second field, because the colour the fog is seen as is not the one it
fogs toward (decision 3). `reach_opacity` is the share of a surface's colour
that fog replaces at the reach, 0 to 1. All three have a `Default` and none is a
Voxel quantity, so a Look pack in step 9 overrides them as it would any plain
values, and nothing crosses the WASM boundary per Voxel (ADR-0001).

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
at 768 m and 66% at 3 km (0.85 was the first default; the kept 0.88 gives 0.5%,
1.8%, 7.2% and 70%), so the full-detail rings are untouched and the fog works on
the coarse ones. Plain `Exponential` is 8.5% at 192 m, which is haze on the
ground the Player is standing on; `Linear` has a kink where it starts. Ticket 03
rendered the first two on the same views and kept this; the table in its Outcome
has the numbers. Which curve is an implementation of the same settings, so the
data does not change if it changes.

**Defaults.** As built (ticket 03): `reach_opacity` 0.88, `sky` (126, 146, 168)
and `color` (136, 160, 190), a blue-grey pair; `HAZE_SKY_SRGB` and
`HAZE_FOG_SRGB` in `render/fog.rs` name them. The text planned `color` as
today's `ClearColor` (43, 44, 47) and 0.85, which keeps 15% of the terrain's
colour at the reach, the "visible but faint" the scope asks for. Ticket 03 found
the dark colour reads as gloom and moved the pair; the Outcome has why.

### 3. The fog colour and the sky colour are a pair, not one value

The text planned one value: `ClearColor` from `FogSettings::color`. The build
found that holds only for a dark colour (ticket 03's Outcome has the numbers):
the tonemapper darkens a bright fully fogged fragment and the clear colour skips
it. So `FogSettings` has two fields. The Client inserts `ClearColor` from `sky`,
in the plugin and so before any other system, and the cameras fog toward
`color`, the value that, once tonemapped, comes out as `sky` within 1/255 per
channel (measured offscreen, terrain against sky). The pair is two named
constants in `render/fog.rs` with a test that pins them and that the fog colour
is brighter than the sky in every channel. The look is what is matched, and it
had to be matched by looking.

The dev-only `TESSERAE_SCREENSHOT_MAGENTA_SKY` probe replaces `ClearColor` after
that, in `capture.rs`, so it keeps working: the fog colour is untouched, the sky
is magenta, and a fogged fragment is still not magenta, which is what the probe
counts.

A Look pack that changes `color` must change `sky` with it. Deriving one from
the other needs the tonemapper's LUT on the CPU, which is step 9's.

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

`TESSERAE_FOG_OPACITY=<float>` overrides `reach_opacity` where the plugin
builds the resource, also under `dev` only. Ticket 03 measured the fog and sky
pair by hand-editing `fog.rs` to force a nearly opaque fog, and ticket 05 could
not repeat that measurement because editing source for a measurement is not a
change anyone keeps. The knob makes the dense-fog probe a run and not a diff.
`FogSettings::default()` stays plain data, so the tests that pin the defaults
are unaffected.

### 6. What a Mod or a Look pack does

Nothing per Voxel. `FogSettings` is two colours and an opacity that step 9's
Look-pack API sets once at load, in the post-process settings DESIGN.md already
lists ("fog parameters"). The derivation from `view_distance_m` and
`lod_distance_m` stays in the engine, so a pack gives colours and an opacity and
not a density that is right for one reach only.

The text left re-deriving `ClearColor` to step 9, on the grounds that nothing
changes the settings at run time yet. That is now done here instead:
`FogSettings` is live data. `follow_settings` watches the resource and, on a
change after startup, writes `ClearColor` from `sky` and rewrites every
camera's `DistanceFog` through `distance_fog` with the current
`StreamingSettings`. So a pack that sets the resource at load — or at any later
point — needs no further engine work, and the cameras do not have to be
respawned. The system early-outs on `is_changed()` and skips the run on which
the resource is inserted, so it costs nothing on an ordinary frame and leaves
the capture probe's own `ClearColor` alone.

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
  fully fogged horizon matches it to 1/255. The colour itself changed (decision
  3). Out of scope by the request.
- **No Look-pack API.** The numbers are data with a `Default`; setting them is
  step 9's.
- ~~**`ClearColor` is not re-derived at run time** when `FogSettings` changes.~~
  Closed: `follow_settings` in `render/fog.rs` re-derives `ClearColor` from
  `sky` and rewrites every camera's `DistanceFog` whenever `FogSettings`
  changes after startup. It early-outs on `is_changed()`, so it does not touch
  the cameras on an ordinary frame, and skips the run on which the resource is
  inserted (`is_added()`) so the capture probe's `ClearColor`, inserted after
  this plugin, is not clobbered on the first frame.
- **The pair is measured, not derived.** `HAZE_FOG_SRGB` is right for this
  tonemapper and exposure; a change to either needs a re-measure, and a pack's
  own colour needs its partner found the same way until step 9 derives it. The
  re-measure no longer needs a source edit: `TESSERAE_FOG_OPACITY` forces the
  dense fog it is read at (decision 5). ~~It has not been re-measured since #15
  landed.~~ It has now: fully fogged terrain (126, 145, 169) against sky
  (126, 146, 168), the same to a level as before #15, in Post-merge
  measurements.
- **Fog is uniform in direction.** No height fog and no directional glow
  (`directional_light_color` is `NONE`), so there is no sun glow around the sun.
- ~~**A blended surface is fogged only if its shader goes through
  `apply_fog`.**~~ Settled by reading `feat/blended-pass`, which is the branch
  that adds the blended pass: its blended surfaces are the **same**
  `ChunkMaterial = ExtendedMaterial<StandardMaterial, ChunkMaterialExtension>`
  as the opaque ones, differing only in `alpha_mode: AlphaMode::Blend`, and its
  `chunk_material.wesl` still ends in `main_pass_post_lighting_processing`,
  which is where `apply_fog` runs. So blended water and glass are fogged by
  construction, on every LOD camera, with no further work on either branch.
  What is unverified is only a capture of the two together, because that branch
  is not merged.
- **The far plane still clips at `lod_distance_m` plus a Cell.** At 88% fog the
  clip edge is a step of the remaining 12%, which is the "faint" the reach is
  allowed to show. ~~Ticket 03 measured it as within 3 of the sky at the very
  far edge.~~ Re-measured after #15 from 300 m: the cut shows as two flat
  silhouette plateaus, and the step across it is (+2, +5, -1) at the farther one
  and (+5, +11, -4) at the nearer one, against (+24, +59, -43) unfogged. So
  "within 3" holds only at the farthest cut; the edge is faint but can still be
  made out. The numbers and the crops are in Post-merge measurements. A higher
  `reach_opacity` hides it and costs the visible 4 km.

## Coordination

Four other branches exist. `git worktree list` and their diffs at the start
of this work:

- `tech/lod-coarse-light-ticket` (lighting the coarse levels): a ticket only. It
  will touch `server/lod.rs` and possibly `chunk_material.wesl`. This branch
  touches neither.
- `claude/friendly-agnesi-4851c8` (blended draw call for water and glass), since
  committed as `feat/blended-pass`: it changes `upload_meshes` in
  `client/chunk_view.rs`, which this branch does not, and it lists distance fog
  as out of scope in its ticket 06. This branch edits `spawn_camera` in the same
  file, a different function. Both branches are on `9ec5571`, and the files they
  both touch are `client/chunk_view.rs`, `render/mod.rs`, `CONTEXT.md` and
  `DESIGN.md` — not `chunk_material.rs` or `capture.rs`, which only that branch
  edits. Whoever merges second resolves about three regions: the `use` block and
  `spawn_camera` in `chunk_view.rs`, and the module list in `render/mod.rs`.
  Nothing in its blended pass needs fog work (see the loose end above).
- `claude/brave-babbage-b12e1c` and `origin/tech/coarse-lod-durable-edits`
  (coarse LOD edits and emitters): a ticket only, server side.
- The tree is rebased on `main` before the PR. It was, at ticket 05: `main`
  moved during the work to `9ec5571`, which merged #15 (coarse levels lit by a
  covered-column rule, editing `server/lod.rs`, `DESIGN.md`,
  `docs/architecture.md` and ADR-0011). The rebase was clean, and the check
  after it is in the Outcome. Tickets 03 and 04 ran before it, on unlit coarse
  levels, and ticket 04's baseline is the `main` before #15.

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
original decision 3, holds for the dark colour only; decision 3 above is the
corrected text.

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

### Ticket 05: docs, the rebase, and the check after it

**Docs.** `CONTEXT.md` gains **Distance fog** in the Space section, `DESIGN.md`
a "Fog" bullet under LOD (with the frame-cost finding in step 6 and
`FogSettings` named in the Look packs list) and `docs/architecture.md` the
`render/fog.rs` entry. The text of this spec above was corrected where the build
disagreed: the `FogSettings` shape (`sky`), decision 3 (a pair, not one value),
the defaults (0.88 and the pair), the curve figures and the far-plane note.
Decisions 4 and 5 (the same fog on every camera, the `TESSERAE_FOG=0` switch)
held as written.

**No ADR.** Nothing here is hard to reverse: the fog is plain data with a
`Default`, the colours are two named constants with a test, the curve is one
line in `distance_fog`, and nothing is stored or crosses the WASM boundary. The
one decision a later reader might want a record of, that the sky's flat colour
changed, is in decision 3 and the Outcome above.

**After the rebase onto `main` at `9ec5571` (#15, coarse levels lit).** The
rebase was clean. Tickets 03 and 04 ran before it, so the views were captured
again with the same knobs as ticket 03 (`FLY_AFTER_S=30`, the 150 m, near-ground
and default-vantage views, fog on and `TESSERAE_FOG=0`, and the tinted 150 m
view), compared with the same views from before it.

- _The fog is unchanged._ Fog share by row, the same-pixel ratio of the channel
  furthest from the sky between a fogged and an unfogged capture, over the rows
  of the 150 m view: before 0.66, 0.59, 0.47, 0.38, 0.23, 0.12, 0.06, 0.02, 0.01
  and after 0.64, 0.58, 0.47, 0.37, 0.21, 0.11, 0.05, 0.02, 0.02 (rows 436 to
  446, 446 to 456, 456 to 470, 470 to 490, 490 to 520, 520 to 560, 560 to 640,
  640 to 800, 800 to 1000). The level 5 edge rows (433 to 445) have the same
  median pixel to within 2 levels, and the clear colour is (126, 146, 168) in
  both. The fogged captures differ pixel by pixel (10% of pixels by more than 6
  levels, before against after), but so do two fogged captures of the same view
  on the rebased branch (10.6%), so that is capture noise and the unfogged pair,
  which does not show it, is the stable one.
- _The coarse light changed almost nothing in these views._ The unfogged 150 m
  capture differs from before in 0.17% of pixels by more than 6 levels and the
  default-vantage one in 0.05%: open terrain is lit as it was, since the rule
  lights a face open sky unless it looks into air well under the terrain. The
  tinted 150 m view differs in 0.14% of pixels by more than 20, and the
  near-ground view in 0.11%.
- _The horizon, the tint and the near ground read as before._ At 150 m the
  fogged horizon dissolves into the sky and the unfogged one is a hard edge; the
  tinted rings (green, salmon, yellow, light green, cyan, lilac) are still told
  apart through the fog; the near-ground view has no coarse block over the
  level-0 horizon. The tinted view shows ring edges as vertical steps at the
  centre column (x = 960), the Eye's column, because the rings' edges are
  lattice-aligned and the view runs along one; they are in the captures from
  before the rebase and in the unfogged ones, so they are ring joins and not the
  fog.
- _The pair was not re-measured at full fog._ Ticket 03's measurement needed a
  throwaway density override in `fog.rs`, and `fog.rs` was not to change in this
  ticket. What argues the pair still agrees: the sky colour, the fog colour and
  the tonemapper are unchanged, a fully fogged fragment is the fog colour
  whatever the light on it, and the fogged level 5 edge rows are identical to
  before. If a later change touches the tonemapper or the exposure, re-measure
  — with `TESSERAE_FOG_OPACITY`, added afterwards (decision 5) exactly so that
  this needs no source edit. That re-measure has since been done, with that
  knob, and the pair holds: Post-merge measurements below.

### Post-merge measurements

Four measurements taken after the rebase onto `main` at `9ec5571` (#15, coarse
levels lit), on a machine quieter than ticket 04's. Apple M5, `dev` build,
`TESSERAE_SCREENSHOT_OFFSCREEN=1`, 1080p; images, logs and the analysis scripts
are in `/tmp/tesserae-fog-review2/` and not committed.

**1. The fog and sky pair, re-measured after #15.** Ticket 05 left this open
because the measurement needed a throwaway edit of `fog.rs`;
`TESSERAE_FOG_OPACITY` (decision 5) is what turns it into a run. One view,
captured three times: the default vantage lifted 300 m
(`TESSERAE_SCREENSHOT_LIFT_M=300`, `FLY_AFTER_S=30`, Eye (0, 375.8, -72)), at
the default density, at `TESSERAE_FOG_OPACITY=0.99999`, and at `TESSERAE_FOG=0`.
All six levels were in (`capture_reach_m 1:325-415, 2:840-984, 3:1608-1906,
4:3144-3752, 5:4168-5292`; level 0 is under the Eye and out of frame at that
altitude) and the three poses agree to 0.01 m, so the captures are pixel for
pixel comparable.

Which terrain counts as fully fogged is read from the **default** capture and
not from the dense one, so the selection cannot assume the answer: per pixel the
display share is `(unfogged - fogged) / (unfogged - sky)` in the channel
furthest from the sky, and a pixel is taken when that share is at least 0.85 and
its unfogged channel is at least 30 levels from the sky. The display share is a
floor for the radiance share, because the tonemapper compresses what fog leaves
(ticket 03), and the dense density is 2.33 times the default, so a radiance
share of 0.85 there is `1 - 0.15^5.43` = 0.99997 here: those pixels sit at the
fog's asymptote to far under one 8-bit level. 11571 pixels qualify, in rows 238
to 494. In the dense capture, 8-bit sRGB:

| Pixels                               | n      | median          | p5              | p95             |
| ------------------------------------ | ------ | --------------- | --------------- | --------------- |
| fully fogged terrain (share >= 0.85) | 11571  | (126, 145, 169) | (125, 144, 168) | (126, 145, 169) |
| sky                                  | 393041 | (126, 146, 168) | (126, 146, 168) | (126, 146, 168) |

**The pair agrees to (0, -1, +1), inside the bar of 3/255**, and every fully
fogged pixel is within one level of its median. The same terrain unfogged is
(126, 177, 104). Loosening the cut to 0.80 (24123 px) and tightening it to 0.90
(4400 px) give the same three medians, so the figure does not depend on where
the cut is put. These are ticket 03's numbers to the level: it read terrain
(126, 145, 169) against sky (126, 146, 168) before #15. The magenta probe below
agrees independently: 21 pixels of that capture equal the plugin's sky value,
and all of them are terrain near the horizon.

**2. The capture probe after `follow_settings`.** The risk is that
`follow_settings`, which rewrites `ClearColor` whenever `FogSettings` changes,
clobbers the probe's magenta on the frame the resource is inserted. The unit
tests cover it; this is the same probe in the real app, at the default density,
over three views — the 300 m view above, the default vantage, and straight down
from 150 m.

| View                    | exact (255, 0, 255) | sky median    | under terrain |
| ----------------------- | ------------------- | ------------- | ------------- |
| 300 m, `FLY_AFTER_S=30` | 488827              | (255, 0, 255) | 0             |
| default vantage         | 290892              | (255, 0, 255) | 0             |
| straight down, 150 m    | 0                   | all terrain   | 0             |

"Under terrain" counts near-magenta pixels (R > 200, G < 60, B > 200) below the
topmost solid pixel of their own column, which is ticket 03's count; the 300 m
view also has none inside a terrain mask taken from its own unfogged capture and
eroded by 3 px, so silhouette edges are excluded rather than explained away. The
counter sees a failure: one magenta pixel painted into the 300 m capture is
counted as one. **The sky is still the probe's magenta and no clear-colour pixel
sits under terrain**, so live `FogSettings` did not break the probe.

**3. Frame cost on a quiet machine.** Same method as ticket 04 —
`scripts/frame-runs 1` per run so the arms interleave, the default flight,
`main` at `9ec5571` built in its own detached worktree under
`/tmp/tesserae-fog-review2/` with its own `target/` and removed afterwards —
repeated 13 times over three arms, 39 runs. `uptime` was read before and after
every run. Median of each figure over the kept runs, range in brackets, in ms:

| Arm             | median_flight_ms | p99_flight_ms  | worst_flight_ms | worst_tick_ms | kept | left out |
| --------------- | ---------------- | -------------- | --------------- | ------------- | ---- | -------- |
| main            | 5.6 (5.0-6.0)    | 7.7 (6.9-9.1)  | 10.0 (7.7-36.5) | 0.9 (0.8-1.0) | 11   | 2        |
| branch          | 5.6 (5.0-8.5)    | 7.5 (6.9-19.5) | 8.8 (8.0-50.4)  | 0.9 (0.8-1.4) | 12   | 1        |
| branch, `FOG=0` | 5.2 (5.0-6.0)    | 7.3 (6.9-8.3)  | 8.8 (7.2-22.0)  | 1.0 (0.8-1.4) | 10   | 3        |

Sorted `median_flight_ms` of the kept runs: main 5.0 5.0 5.0 5.1 5.5 5.6 5.6 5.7
5.8 5.8 6.0; branch 5.0 5.0 5.0 5.1 5.3 5.6 5.6 5.7 5.7 5.8 5.9 8.5; `FOG=0` 5.0
5.0 5.0 5.0 5.0 5.5 5.6 5.6 5.8 6.0. The one 8.5 is the branch's first run,
taken while the machine was still settling from the baseline build.

_Left out._ The same rule as ticket 04: a run whose median flight frame is at or
near the 60 Hz cap. main 16.6, 16.6; branch 16.6; `FOG=0` 16.6, 16.6, 16.5. That
is 6 of 39 runs (ticket 04 lost 13 of 42), spread over all three arms.

_Load._ The driver waited for a 1-minute load under 3 before each run, giving up
after two minutes. 32 of the 39 runs started under 3; the load before was 2.48
to 5.02 (median 2.83) and after 2.88 to 6.63 (median 3.56). Ticket 04 could not
get below about 4. The last three rounds all read 5.0 ms on all three arms,
which is what a quiet machine looks like here.

_Paired within a round_, which is the comparison the interleaving is for, over
the rounds where both arms were kept: branch - main is 0.00 ms at the median
(n = 10, each round between -0.3 and +0.2, bootstrap 95% interval of the median
[-0.20, +0.10]); branch - `FOG=0` is +0.05 ms (n = 10, -0.2 to +0.3, interval
[-0.10, +0.20]); `FOG=0` - main is 0.00 ms (n = 9, interval [-0.10, +0.00]).

**Fog's cost is still not measurable, and the bound is now about 0.2 ms**, down
from ticket 04's 0.3 ms. That is a bound and not a measurement: the paired
differences sit on either side of zero, so nothing here says fog costs anything
at all, only that it cannot cost more than about 0.2 ms a frame at 1080p on this
GPU. The worst tick is 0.9, 0.9 and 1.0 ms, so no main-thread or Server work was
added, as expected. The branch also runs `follow_settings` on every frame, an
early-out on an unchanged resource: it is inside the branch - main difference
above and **cannot be seen** — 0.00 ms at the median, with the whole six-camera
fog in the same figure.

**4. The far edge at the reach, after #15.** The same 300 m view. Terrain ends
at `lod_distance_m` plus a Cell, and the end is flat: the silhouette is two
horizontal plateaus, row 235 over columns 960 to 1232 and row 256 over 1221
columns, with almost nothing between them. Medians across each line, 8-bit sRGB,
sky above it and terrain under it:

| Plateau | sky above       | silhouette row  | 1-3 rows under  | 4-11 rows under | step, fogged  | step, `FOG=0`   |
| ------- | --------------- | --------------- | --------------- | --------------- | ------------- | --------------- |
| row 235 | (126, 146, 168) | (128, 150, 167) | (128, 151, 167) | (129, 153, 166) | (+2, +5, -1)  | (+24, +59, -43) |
| row 256 | (126, 146, 168) | (128, 152, 166) | (131, 157, 164) | (131, 160, 163) | (+5, +11, -4) | (+23, +58, -44) |

**Partly confirmed, and the "within 3 of the sky" reading does not hold along
the whole edge.** At the row 235 plateau the step across the cut is within 5
levels (the silhouette row itself, which MSAA mixes with the sky, is within 4),
which is ticket 03's finding to a level. At row 256 it is 11 levels of
green. I did not establish why the two differ: the view geometry puts them
within a few tens of metres of the same distance from the Eye, so the cause is
more likely in what is behind each edge than in the fog, and it was not chased.
Fog takes the cut from a 59-level step to 5 and 11, and the terrain then grows
out of the haze over 40 rows (+23 green by row 293), but at 2x the cut can
still be made out as a faintly lighter rectangle against the sky
(`j4-edge-crop-default.png` beside `j4-edge-crop-nofog.png`). So the edge is
faint but not invisible, which is what the loose end now says.

## Tickets

| #   | Title                                       | Blocked by |
| --- | ------------------------------------------- | ---------- |
| 01  | The fog numbers and their derivation        | -          |
| 02  | Fog on every camera, matched to the sky     | 01         |
| 03  | Tune the fog against captures               | 02         |
| 04  | Frame cost against main                     | 03         |
| 05  | Docs and the PR                             | 03, 04     |
