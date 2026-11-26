# 07: Cloud layer

Status: ready-for-agent
Blocked by: 05

Blocked by 05 only because both amend the `sky` record.

Spec: [Day and night cycle](../spec.md).

## What to build

Clouds drift across the sky at the Game's altitude.

- `sky` gains the Cloud layer (altitude, thickness, cover, wind direction and
  speed); `biome-def` gains a cloud cover scale. The data-only TOML, the
  starter's contract and the compatibility doc change alongside. The terrain
  band's top becomes a public helper of the generator for the altitude check.
- The Server sends the Eye's Biome cover scale with the sky and fog pair,
  blended like it; the Client eases it.
- A translucent slab at the altitude and thickness, following the Eye, out to
  the reach, in the blended pass, casting no shadow, faded by Distance fog.
  Inside it the view fogs toward the cloud colour.
- 2D noise in metres, thresholded by the cover, a few stacked samples for
  thickness, offset by the wind times the time since day 0, wrapped on the CPU
  to the noise period. Lit by the Sky state's light and ambient; hides the
  stars behind it.
- Terra: base 300 m, 30 m thick, cover 0.4, a slow wind; Dry and Savanna scale
  the cover down, Swamp up.

## Acceptance

- [ ] Each rule (base above the terrain band, thickness 1 to 200 m, cover in
      [0, 1], scale in [0, 4], wind at most 50 m/s) is a load error naming the
      field; no Cloud layer draws no clouds.
- [ ] The same Time of day gives the same clouds after a reopen, and the same
      cloud field at another Voxel size.
- [ ] Captures from under the clouds, from above, from inside, at dusk and at
      night, approved by the user.
- [ ] GPU frame time of the Cloud layer on `flight` at noon and midnight,
      interleaved against `main`, at most 0.2 ms over it, recorded in Comments.
