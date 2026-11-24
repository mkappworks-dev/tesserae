# 02: Light balance, aerial perspective and the sky

Status: ready-for-agent
Blocked by: 01

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 2, for what the art pass leaves, smallest lever first, each as a
capture the user approves:

1. A near haze in metres in Distance fog, independent of the reach, fading
   blue first, beside the reach-sized fade. Update the glossary's Distance fog
   entry in the same commit.
2. A sky gradient shader: the Biome's sky colour at the zenith blending to its
   fog colour at the horizon, replacing the flat clear colour, with a sun disc
   and soft glow at the fixed light's direction. Judged per Biome on its
   `{biome}-far` capture.
3. Ambient and Sky light balance, judged on the jungle and swamp floors.
4. Whether block light follows `Exposure`, on the cave view.
5. AgX beside TonyMcMapface; a change re-measures the fog's tonemap darkening
   constant.
6. `post_saturation` only if the picture is still loud.

## Acceptance

- [ ] The cause of what remains after 01 is named from captures, not guessed.
- [ ] The mid-distance village and jungle views at 70 and 110 m fade and cool
      with distance; approved.
- [ ] `jungle-ground-close` and `swamp-forest-ground` floors readable; approved.
- [ ] Block light's relation to `Exposure` decided on `cave`.
- [ ] The sky reads blue with a lighter horizon and a sun in every Biome's
      `{biome}-far`; the terrain meets the horizon with no edge; approved.
- [ ] Fog tests pass, including at a different Voxel size.
