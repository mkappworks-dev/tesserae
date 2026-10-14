# Fog on every camera, matched to the sky

Status: ready-for-agent
Blocked by: 01

Spec decisions 1, 3, 4 and 5, and the "rests on" facts: no shader change.

## What to build

- Attach the same `DistanceFog` (from ticket 01, built once from
  `StreamingSettings` and `FogSettings` and cloned) to the near camera and to
  every coarse camera in `spawn_camera`, `client/chunk_view.rs`. Keep the edit
  to that function, because the blended-pass session is editing `upload_meshes`
  in the same file.
- A plugin in `render/fog.rs` (add it to `render::plugin`) that initialises
  `FogSettings` and inserts `ClearColor` from its colour. It runs at build time,
  so the dev probe `TESSERAE_SCREENSHOT_MAGENTA_SKY` in `capture.rs`, which
  inserts its own `ClearColor` later in `client::plugin`, still wins. Check the
  order and the probe; do not edit `capture.rs` unless it does not.
- Under `dev`, `TESSERAE_FOG=0` attaches no `DistanceFog` (spec decision 5). Not
  a zero opacity.
- `chunk_material.wesl` and `chunk_material.rs` are **not** edited. If a capture
  shows the fog does not reach the chunk material, find out why from the Bevy
  sources before touching either, and say what was found.
- The clear colour against the fog colour, measured, not assumed (spec decision
  3): capture the same view with the fog density forced very high (a throwaway
  local edit, not committed) and compare the fully fogged terrain pixels with
  the sky pixels. Report both as 8-bit sRGB. If they agree within 3/255 per
  channel, keep one value. If not, set `ClearColor` to what the tonemapper makes
  of the fog colour, as a named constant with a one-line comment, and a test
  that pins it.

## Acceptance criteria

- A test builds the app's cameras as `camera_app` does and asserts every camera
  that draws terrain (the near one and each coarse one) has a `DistanceFog`, and
  that all of them are equal field by field.
- A test that `ClearColor` is the fog colour in the default app.
- A test that `TESSERAE_FOG=0` leaves no camera with a `DistanceFog`, if the
  switch can be tested without setting the process environment; if it cannot,
  the switch is a function of a plain argument and that is tested.
- A first capture, looked at: fog visibly on the distant terrain, none on the
  ground at the Player's feet. Also a capture with `TESSERAE_FOG=0` from the
  same binary. Both in `/tmp/tesserae-fog-review/` only. Say what each shows.
- `git diff main --stat` shows no image and no change to `chunk_material.*`.
- `./scripts/cargo-ci` passes.
- `Status: done` is set in the commit that finishes this ticket.

## Comments
