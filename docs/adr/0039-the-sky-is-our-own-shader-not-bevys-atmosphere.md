# The sky is our own shader, not Bevy's Atmosphere

Status: accepted.

The day and night cycle draws the sky in Tesserae's own shader: the gradient
from a Biome's sky colour at the zenith to its fog colour at the horizon, with a
sun and moon that move, the Moon phase, procedural stars and a tint from the Day
curve, on a camera with no HDR. Night is uniforms (the light's direction, colour
and strength, the ambient light, the sky and fog colours), never a relight or a
remesh, because Sky light is how much of the sky a face sees, not how bright it
is.

## Considered Options

Bevy 0.20's `Atmosphere` was rejected for now:

- `AtmosphereSettings` requires `Hdr` on the camera, and the sky it draws is in
  physical units, so the sun's illuminance and `Exposure` would be retuned
  together, probably with `AutoExposure` (also HDR-only).
- It replaces the clear-colour sky. Distance fog hides the square edge of the
  streamed reach only because the fog colour is the colour that comes out as the
  sky; with the atmosphere drawing the sky that pairing is gone, and its own
  aerial perspective (32 km by default) hazes the terrain on top of Distance fog
  rather than hiding a 6 km edge.
- It draws no stars, moon phases or clouds; each would still be ours.
- It costs a compute pass of four LUTs and a full-screen pass every frame on the
  world camera.

## Consequences

- `Atmosphere` can return as a high-quality option once a camera is HDR for
  other reasons (the Look overhaul), with the fog edge solved for it then.
- Night brightness is a display-referred curve the Day curve declares, not
  physical lux: at a fixed exposure Bevy's full moon (0.05 lux against a 10 000
  lux sun) would be invisible.
