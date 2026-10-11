# Document the controls, and make the hot-reload promise true

Status: done
Blocked by: 04

## Task

Step 2 adds the first interactive surface in the engine and nothing says what
its keys are. Write them down in CONTRIBUTING.md, beside the quick start, where
someone who has just run the binary will be.

Two gaps turned up while writing them:

- **No zoom and no way home.** Scroll did nothing and flying away from the grid
  had no return. Scroll now sets movement speed rather than field of view — in
  a fly camera the distance to what you are looking at is what you want to
  control, and `W`/`S` already do that. `R` returns to the spawn pose, held in
  a `CameraHome` component so the Server can own the spawn point from step 3.
- **CONTRIBUTING.md promised hot-reload that did not work.** The dev-loop table
  said a `.wesl` edit "hot-reloads in the running app", and that `file_watcher`
  was "enabled in the dev profile". Neither watcher feature is a Bevy default,
  neither was enabled, and `file_watcher` is a cargo feature, not something a
  profile can switch on.

## Done when

The controls are documented, and editing `chunk_material.wesl` while the app
runs reloads it without a restart.

## Notes

A `dev` feature turns on `bevy/embedded_watcher`, not `file_watcher`. The
engine's own shaders are embedded in the binary as `bevy_pbr`'s are, so
`embedded_watcher` is what watches them.

`file_watcher` is deliberately left out rather than enabled alongside: it
watches an `assets/` directory, and there is none. Nothing is loaded from disk,
the release workflow packages the bare binary, and where assets ship is open
until the look-pack API in step 9. Enabling it today only logs two warnings.
The doc says so rather than implying the asset half works.

Verified by breaking the shader while the app was running: the log showed
`Reloaded embedded://tesserae/render/chunk_material.wesl` followed by the
pipeline rejecting the bad syntax, with no restart.

## Comments
