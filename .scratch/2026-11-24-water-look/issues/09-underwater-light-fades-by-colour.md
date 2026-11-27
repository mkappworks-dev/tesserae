# 09: Underwater light fades by colour

Status: done
Blocked by: 08

Spec: [Water that looks natural](../spec.md), Underwater.

## What to build

Under water, red fades first and blue last. A Fluid on the water path uses
Bevy's atmospheric fog falloff: extinction per channel from its absorption and
in-scattering from its scatter tint, with the engine defaults. Its range comes
from the extinction, not from dimming; the clear colour is the in-scattered
colour at full extinction; the colour branch's haze does not apply under water.
Other fluids keep today's single-hue fog.

## Acceptance criteria

- [x] Underwater tests: water's fog extinguishes red faster than green faster
      than blue; lava's look is unchanged; the existing tests pass.
- [ ] Before and after captures from the underwater fixture, shown to the user.

## Comments

- 2026-11-27, on df7ec922 plus this ticket:
  - Under a Fluid with a water look, the terrain cameras take Bevy's
    `FogFalloff::Atmospheric`: extinction per channel is the look's absorption
    plus 0.01 m⁻¹ of scattering, and in-scattering equals it, so what is lost
    is put back as the scatter tint. Pure water's absorption alone leaves blue
    at 16 % 200 m off, past the streamed reach; with the scattering it is 2 %,
    and a clear lake's floor still shows some 50 m away. A first try at
    0.04 m⁻¹ hid the floor 30 m off.
  - The clear colour is the scatter tint as sRGB, shaded by the hour's ambient
    as before. The chunk shader takes the Atmospheric branch through bevy_pbr's
    `atmospheric_fog`, and skips the colour work's haze there. Lava and any
    Fluid without a water look keep the single-hue fog.
  - Pure water's scatter tint is now linear `[0.0, 0.03, 0.16]`: with more
    green, green's in-scattering filled in long before blue's and the middle
    distance came out teal.
- Captures: `/tmp/tesserae-water-look-captures/09-before-underwater.png` (08's
  look) and `09-after/underwater.png`. The floor near the Eye keeps some of its
  colour and fades to blue with distance; the old fog's 160 m view of the floor
  is shorter now. They wait on the user's review.
