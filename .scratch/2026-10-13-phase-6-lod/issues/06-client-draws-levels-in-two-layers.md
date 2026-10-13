# The Client draws levels, in two layers

Status: done
Blocked by: 04

Spec decisions 7 and 8. Read `bevy_camera-0.20.0`, `bevy_core_pipeline-0.20.0`
(`prepare_core_3d_depth_textures`) and `bevy_render-0.20.0`
(`prepare_view_targets`) under `~/.cargo/registry/src/*/` before writing it.

## What to build

- `chunk_view.rs`: the entity map is keyed by `(level, ChunkCoord)`; a handler
  for `LodMeshReady` / `LodUnloaded` shares the upload and despawn code with
  level 0. A level-`k` entity sits at `coord * cell_size * 2^k`, its mesh is
  uploaded with `voxel_size * 2^k`, and it carries `MeshTag(k)`, `RenderLayers`
  {1} (level 0 stays {0}) and, for `k >= 1`, `NotShadowCaster`.
- Cameras: the existing camera becomes the near camera (`order 1`,
  `clear_color: None`, layers {0}) with a `MainCamera` marker, and the three
  `With<Camera3d>` queries (`capture.rs` x2, the `chunk_view.rs` test) filter by
  it. A far camera is spawned as its child with an identity transform:
  `order 0`, default clear, layers {1}, same FOV, `near = view_distance_m / 4`,
  `far = lod_distance_m + Cell`. The directional light is on both layers. Check
  that UI text still lands on the near camera.
- Shader (`chunk_material.wesl`): read `mesh_functions::get_tag`; push the
  clip-space position away from the camera along the view ray by
  `BIAS_FRACTION * voxel_size * 2^level` metres, lighting with the unshifted
  position. `BIAS_FRACTION` is a named constant with a comment.
- Debug tint: a uniform on `ChunkMaterialExtension` (binding 101) that tints by
  level, set from `TESSERAE_LOD_TINT` under `dev`.
- Capture: knobs to see far, `TESSERAE_SCREENSHOT_PITCH_DEG` and
  `TESSERAE_SCREENSHOT_LIFT_M` (a climb within the Server's speed ceiling, not a
  teleport).

## Tests

- `chunk_mesh_to_mesh` scales by the level's Voxel size (extend the existing
  test).
- Entity placement and scale per level; an empty mesh despawns; a replaced mesh
  drops its old handle.
- The far camera is a child of the main camera and its projection `far` covers
  `lod_distance_m`.
- Level 0 entities are on layer 0 and level `k` on layer 1 with `MeshTag(k)`.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- `cargo dev` runs and a capture shows terrain past 192 m. Verify by looking at
  the image, not by exit code. If the capture is black, use
  `TESSERAE_SCREENSHOT_OFFSCREEN=1`; do not debug the renderer.

## Comments

Built as specified except where listed. Tests: 10 new in `chunk_view.rs` and
the rest moved to the `(level, coord)` key (placement and scale per level,
layers and `MeshTag`, `NotShadowCaster` only above level 0, replaced and
emptied coarse meshes free their handle, unloading one level leaves the others,
the far camera is a child at the identity with `far >= lod_distance_m`, the near
camera orders last, clears nothing and owns the UI, the sun is on both layers),
one in `chunk_mesh.rs` (a level's own Voxel size scales the mesh), one in
`world.rs`, one in `capture.rs` (the climb stays under the Server's ceiling).
`./scripts/cargo-ci` passes.

What the spec read from the Bevy 0.20 sources, checked against them and against
captures:

- **Shared target, per-camera depth clear: true.** `prepare_view_targets` keys
  the main colour texture by target, format, usage and MSAA;
  `prepare_core_3d_depth_textures` keys the depth texture by target and MSAA and
  gives each camera its own `ViewDepthStencilTexture` with its own first-use
  clear. A `ClearColorConfig::None` camera of higher order draws over the lower
  one and `MsaaWriteback::Auto` carries the MSAA contents across. The captures
  show both layers in one image, the near layer over the far one, no black or
  doubled picture. The depth clear per camera is read from source, not isolated
  empirically.
- **`far` is only the culling plane: true.** `PerspectiveProjection` builds
  `perspective_infinite_reverse` from `near` alone; `far` goes to
  `compute_frustum`. `far` is `lod_distance_m` plus a level-0 Cell (32 m). The
  outermost ring can reach past `lod_distance_m` (it is cut on its anchor Cell,
  so up to a level-5 Cell, 1 km, further on one side); the far plane trims that
  to a uniform horizon, which is what is wanted.
- **WRONG: "the far camera's cascades have nothing to render".** A directional
  light's `RenderLayers` is both the set of views it lights and the mask of
  shadow casters for every one of those views (`check_dir_light_mesh_visibility`
  uses the light's mask, `queue_shadows` the shadow view's, which is the
  light's). A light on {0, 1} therefore also puts every level-0 caster into the
  far camera's cascades: a second shadow pass over level 0 per frame, drawn
  into the same depth-array layers (reset per camera, so it is correct, only
  doubled). Not measured. The fix if ticket 07 finds it matters is two lights
  with the same transform, {0} with shadows and {1} without; the coarse layer
  has no cascades to render and nothing near enough to be shadowed by level 0.
- **UI: lands on the near camera, with a caveat.** `DefaultUiCamera` takes the
  highest-`order` camera whose target is the primary window, which is the near
  camera, so a window run needs nothing. An image target is never a candidate,
  so under `TESSERAE_SCREENSHOT_OFFSCREEN` the UI would have had no camera;
  `IsDefaultUiCamera` on the near camera fixes it, and offscreen captures now
  show the readout (by the source they had none before). Seen in both modes.
- **Both cameras need the same `RenderTarget`.** Not in the spec: the offscreen
  capture put only one camera on the image and the far layer would have gone to
  the window. `render_offscreen` now sets every `Camera3d`.
- **`MeshTag` and the clip-space push work.** `get_tag` returns the tag, the
  tint is by level, and a `BIAS_FRACTION` of 2.0 in place of 0.2 visibly gives
  the finer level the pixel where two coarse levels overlap (more yellow over
  green, more red over yellow in the same pose), so the push reaches the depth
  test. The value stays at 0.2 for ticket 08.

Where the spec was silent:

- The shader needs the level-0 Voxel size to turn a fraction into metres and a
  mesh does not carry it, so binding 101 is a `LodParams { voxel_size, tint }`
  struct and not a bare tint flag. `ChunkMaterialExtension::default()` has
  `voxel_size: 1.0`; `add_material` sets the World's.
- The tint is `TESSERAE_LOD_TINT` (anything but `0`), `dev` only, and replaces
  55% of a Material's colour with a hue per level: red 1, yellow 2, green 3,
  cyan 4, violet 5. Level 0 is untinted.
- `WorldSettings` gained `chunk_origin_at(coord, level)` and
  `voxel_size_at(level)`; `chunk_origin` is now level 0 of the first and has no
  non-test caller, so it carries the same `allow(dead_code)` as `chunk_at`.
- Level 0 entities carry `MeshTag(0)` and layer 0 explicitly, so the rule is
  one code path and a test can read it.
- The two `allow(dead_code)` on `LodMeshReady` and `LodUnloaded` are removed;
  the edit is in `server/streaming.rs` and went in with ticket 05's commit
  because both of us had that file open.
- Capture: pitch is set on the first frame; the lift is spread over the flight
  and held to what the Server's ceiling leaves after the horizontal speed (it
  clamps rather than errors, so a bigger lift wants a longer
  `TESSERAE_SCREENSHOT_FLY_S`). The captures below also used a throwaway yaw
  knob that is not in the code.

What the captures show (window and `TESSERAE_SCREENSHOT_OFFSCREEN=1`, Apple
M5; the images are in the session scratchpad and are not committed):

- `default.png`, the default view: terrain to a flat horizon, readout and hints
  drawn. `horizon_ground.png`: the same at -3 degrees, offscreen, terrain to the
  horizon with no visible seam at that scale.
- `tint_ground.png`: the same pose tinted. Level 0 (untinted green) ends in a
  chunk-edged line near 192 m, then red, yellow, green, cyan and violet bands to
  the horizon: terrain past 192 m, and rings as rings. `tint.png`,
  `tint_yaw30.png`, `top.png` (675 m up, looking down) are the same from higher.
- **Hairline cracks at ring edges.** Slivers where the clear colour shows
  through, a few pixels wide, at the 2/3 boundary in `tint_yaw30.png`, with a
  sliver of level 4 behind one of them. Not a depth effect (they are the same at
  `BIAS_FRACTION` 2.0, `tint_yaw30_bias2.png`). This is the case spec decision 9
  names for ticket 08.
- **Patchwork where two coarse levels draw the same column.** The hole is one
  Cell tall (half-height 0), so any terrain outside that slab is drawn by the
  level and by the next one up, and the higher surface wins per pixel: red
  inside yellow inside green in `tint.png`, and from 675 m up (`top.png`) levels
  3 and 4 fight over everything below the Eye. The near layer hides this for
  level 0 and 1 only. A larger `BIAS_FRACTION` shifts it towards the finer
  level; a hole as tall as the finer ring is the other fix (ring shape, ticket
  02's, not this one's).
- **A straight seam through the screen centre when the Eye is on a lattice
  line** (`tint.png`, `tint_yaw90.png`): the capture spawns at x = z = 0 and
  flies along an axis, so the Eye sits on the boundary between Chunk -1 and 0
  and the two sides get different coverage. It vanishes at yaw 30
  (`tint_yaw30.png`), so it is the rings and not the cameras. Worth a look in
  ticket 08 before it is blamed on rendering.

Superseded in ticket 08: the far layer became one camera per coarse level and
the depth push was removed, because a push cannot beat a surface that is a
coarse Voxel higher along a grazing ray. Everything above about two layers, the
shared sun and `BIAS_FRACTION` describes the first build.
