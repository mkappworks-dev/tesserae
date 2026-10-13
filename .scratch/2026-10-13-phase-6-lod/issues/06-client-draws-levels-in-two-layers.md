# The Client draws levels, in two layers

Status: ready-for-agent
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
