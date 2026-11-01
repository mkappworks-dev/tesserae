# 10: Experiment with Bevy's occlusion culling

Status: ready-for-agent
Blocked by: 07

Spec: [streaming performance](../spec.md).
PR boundary: experiment branch `tech/streaming-occlusion`; a change merges
only if it wins.

## What to try

Enable Bevy's `OcclusionCulling` on the Player camera if it applies to our
`MaterialExtension` meshes on Metal. Record whether it does, and if so the GPU
and render CPU time in every fixture.

## Acceptance criteria

- [ ] Whether it applies on Metal, with the reason, recorded here.
- [ ] If it applies: adopt only if it passes the spec's effect gate with no
      popping in review captures.

## Comments

### 2026-11-01: applies on Metal once the prepass cuts coverage

- Applies. Bevy 0.20 culls only with a `DepthPrepass` and GPU preprocessing
  in culling mode. The M5 under Metal logs "GPU preprocessing is fully
  supported" (wgpu 30 exposes `INDIRECT_FIRST_INSTANCE` and `IMMEDIATES`).
- Not as it stood. A `DepthPrepass` alone blanked every Chunk, water too.
  `bevy_pbr`'s prepass for an opaque material has no fragment stage
  (`fragment_required` in `prepass/mod.rs` wants `MAY_DISCARD`), so the
  coverage rule never ran there. Coarse levels, which overlap every finer
  level and stand above its surface, wrote the nearest depth; the main pass
  loads that depth, discards the coarse fragment by coverage and fails the
  finer one on `GreaterEqual`.
- Isolated by: depth compare `Always` brings the terrain back; Chunks kept
  out of the prepass render normally; Bevy's default prepass shaders, no hand
  camera, `NoIndirectDrawing` and no view-cache fix all stay blank.
- Built: `TESSERAE_OCCLUSION=1` under `dev` or `profiling` gives the Player
  camera `DepthPrepass` and `OcclusionCulling` (`CullOccluded`, default off).
  `cut_coverage_in_prepass` gives every non-orthographic prepass pipeline a
  fragment stage that runs `finer_drawn`, now in `coverage.wesl` and shared
  with the colour pass. The sun's orthographic shadow views are skipped:
  coarse levels cast no shadow. Switched off, no pipeline changes.
- Captures, `bin-10-cut`, under `/tmp/claude-streaming-impl/review10/`:
  `{stationary,sprint,lift90}-{0,1}` (0 off, 1 on; lift90 is stationary with
  `TESSERAE_SCREENSHOT_LIFT_M=90 TESSERAE_SCREENSHOT_PITCH_DEG=-5`). No false
  occlusion or missing Chunks. Stationary: 0 pixels differ. Sprint: 66, in
  one distant patch. Lift90: the climb is frame-rate dependent (eye 149.92 vs
  149.81 m), so the whole view shifts. A single end capture cannot show
  popping in flight; the final series' captures should look for it.
- Expect it to fail the effect gate. Single runs, not an A/B, frame p50 off
  -> on: stationary 5.03 -> 15.94 ms, sprint-flight 4.92 -> 16.07, lift90
  3.60 -> 15.53. Render thread CPU barely moves (1.4 -> 1.7 ms) while the
  render schedule waits 15.6 ms, so the GPU is the cost. It is the culling
  passes, not the cut: with nothing drawn, `DepthPrepass` alone ran 3.31 ms
  and with culling 15.35 ms; the prepass without Chunks ran 5.18 ms. A GPU
  capture (`--instruments gpu`) would name the pass.
- Status unchanged: the effect gate is pending the final review series.
