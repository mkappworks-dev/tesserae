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

## Tickets

| #   | Title                                       | Blocked by |
| --- | ------------------------------------------- | ---------- |
| 01  | The fog numbers and their derivation        | -          |
| 02  | Fog on every camera, matched to the sky     | 01         |
| 03  | Tune the fog against captures               | 02         |
| 04  | Frame cost against main                     | 03         |
| 05  | Docs and the PR                             | 03, 04     |
