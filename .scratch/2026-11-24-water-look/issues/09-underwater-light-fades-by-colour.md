# 09: Underwater light fades by colour

Status: ready-for-agent
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

- [ ] Underwater tests: water's fog extinguishes red faster than green faster
      than blue; lava's look is unchanged; the existing tests pass.
- [ ] Before and after captures from the underwater fixture, shown to the user.
