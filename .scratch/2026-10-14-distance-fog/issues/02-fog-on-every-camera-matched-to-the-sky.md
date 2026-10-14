# Fog on every camera, matched to the sky

Status: done
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

### Done

`render::fog::plugin` inserts `FogSettings` and `ClearColor`; `spawn_camera`
clones one `DistanceFog` onto the near camera and every coarse camera.
`TESSERAE_FOG=0` (dev) is `fog::enabled()` feeding `camera_fog(enabled, ..)`, so
the switch is tested without the process environment. `chunk_material.*` is not
touched.

Sky against fully fogged terrain, 8-bit sRGB, offscreen capture at 1920x1080
with a throwaway density of 0.05 per metre (not committed): sky (43, 44, 47)
everywhere; terrain from row 600 down is (45, 46, 49) for 46% of its pixels and
at most (46, 47, 50), mean (45.2, 46.2, 49.3). The difference is 2 to 3 per
channel, within the 3/255 bar, so one value stands and there is no constant. The
tonemapper lifts the fog colour by 2.

Captures, all offscreen: the default vantage and a horizon view at the default
height and at 150 m, each with and without `TESSERAE_FOG=0`. Fog is on the
distant terrain in the default-density captures and not at the feet (under 1% of
colour on the bottom third). It is gentle: the terrain at the horizon is still
bright green, because fog mixes in linear radiance before the tonemap, so 30 to
60% of the radiance left is most of the brightness. With the magenta probe the
sky is magenta (255, 0, 255) and no pixel below the horizon is. Whether the
default reads as fog is ticket 03's to judge.
