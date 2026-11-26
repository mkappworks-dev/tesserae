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

- [x] Each rule (base above the terrain band, thickness 1 to 200 m, cover in
      [0, 1], scale in [0, 4], wind at most 50 m/s) is a load error naming the
      field; no Cloud layer draws no clouds.
- [x] The same Time of day gives the same clouds after a reopen, and the same
      cloud field at another Voxel size.
- [ ] Captures from under the clouds, from above, from inside, at dusk and at
      night, approved by the user.
- [ ] GPU frame time of the Cloud layer on `flight` at noon and midnight,
      interleaved against `main`, at most 0.2 ms over it, recorded in Comments.

## Comments

Built; tests written, not yet run (every test runs once at the end of the
spec), and no captures or frame times taken, so the boxes stay unticked.

- Contract: `sky-params.clouds: option<cloud-layer>` (last); `cloud-layer {
  altitude-m, thickness-m, cover, wind-direction-deg, wind-speed-m-per-s }`.
  The direction is where the clouds drift toward, clockwise from north (-Z),
  so 90 drifts east (+X). `biome-def.cloud-cover: option<f32>` after
  `cave-scale`. Data-only: `[world.sky.clouds]` and a Biome row's
  `cloud-cover`. Both WIT copies identical; `full.toml` carries both.
- Core: `SkyParams::clouds: Option<CloudLayer>`, `CloudLayer::drift_m(seconds)`
  (in f64, wrapped to `CLOUD_PERIOD_M`, 20 480 m), `cloud_noise_origin_m(eye,
  drift)` (the Eye less the drift, wrapped in f64). `SkyInputs.seconds` (since
  day 0's midnight; the Client's `ClientTimeOfDay::seconds`), `Air.cloud_scale`
  (eased with the pair); `SkyState.cloud_cover` (cover times scale, at most 1;
  0 with no layer) and `cloud_drift_m`. `Biome::cloud_cover`, saved in
  `SavedBiome` like `cave_scale`; `MAX_CLOUD_SCALE` 4.
- Validation, each naming its field: `sky.clouds.altitude_m` (finite),
  `thickness_m` (1 to 200), `cover` (0 to 1), `wind_direction_deg` (finite:
  not in the spec's list, added so a NaN cannot reach the drift),
  `wind_speed_m_per_s` (0 to 50), and `biomes.cloud_cover` (0 to 4) in both
  `BiomeTable::new` and the Mod loader.
- The base check: `WorldgenParams::terrain_band_m()` is now public (the
  generator's own method calls it). It is checked in `tesserae-mods`'
  `declare.rs` `apply`, right after the Game's `world_params`, through
  `SkyParams::validate_cloud_base(top)`: only there are the Game's `sky` and its
  terrain both known (`sky` is Game-only, as `world` beyond the character is).
  Mods cannot declare landforms, so the top is Sea level + 192 + 12 (228 m for
  Terra). `SkyParams::validate` alone does not check the base.
- Server: `SkyFogTarget.cloud_scale`, from `Worldgen::cloud_scale_at`, which
  blends like `sky_fog_at` (the same selector rule and weights over every
  Biome, a missing scale counting 1); the surface Biomes' even underground.
  Resent when it moves 0.01. The Client eases it in `EasedAir` with the pair.
- Rendering (`render/clouds.rs`, `clouds.wesl`): two quads, the slab's base and
  top, children of the camera with no RenderLayers (layer 0, ADR-0019),
  `NoFrustumCulling`, `NotShadowCaster`, `NotShadowReceiver`, one
  `CloudMaterial` with `AlphaMode::Blend`, no prepass, no shadows, cull none.
  The vertex shader lays each face round the camera out to the reach
  (`fog::reach_m`, now `pub(crate)`) and ignores transforms. Only the face the
  Eye is outside of is visible (both while inside), so a ray is shaded once.
  The fragment takes 4 samples along the ray inside the slab (from where it
  enters, or the Eye, to where it leaves, at most 1500 m), each of 2-octave
  periodic value noise (PCG-hashed lattice, 32 and 128 cells over the period),
  thresholded by the cover; a column that clears the threshold by more stands
  taller about the slab's middle. Alpha is `1 - exp(-0.1/m × density × path)`.
  Lit by the Sky state's light and ambient (shares 0.9 and 0.55, set on the
  CPU), a darker underside and a forward silver toward the light; fogged with
  bevy's `exponential_squared_fog` and the sky's glow at the density-weighted
  distance, then tonemapped in shader; faded out over the last quarter of the
  reach. Hidden with the sky (under water, the probe), with no layer and at a
  cover of 0. The noise never sees a large number: the CPU passes the Eye and
  the wrapped origin under it, the shader adds only the ray's offset.
- Decided: inside the slab the view fogs toward the cloud colour through the
  same integration (rays from the Eye to the face they leave by), not a second
  fog, so crossing is continuous and graded by what is around the Eye. No
  per-pixel jitter (it would shimmer). The cloud colour ignores the camera's
  exposure, as the sky's zenith does; if the Look overhaul changes exposure the
  shares need re-picking. Constants are first values for the captures.
- `tesserae_core::sky` gained a module-level
  `allow(clippy::disallowed_methods)`: it is presentation, and the existing sun
  maths already called `sin`/`asin`.
- Terra: base 300 m, 30 m thick, cover 0.4, wind 3 m/s toward 70°; Dry 0.4,
  Savanna 0.6, Swamp 1.75.
- `client::sky::plugin` and `client::time_of_day::plugin` are `pub(crate)` for
  the render tests.
- Tests: `tesserae-core/tests/sky_state.rs` (no layer has no cover or drift,
  cover times scale capped at 1, the wind's direction, wrap precision after
  3e8 s, the same drift after a reopen, the same noise origin at 1 m and 0.25 m
  Voxels, each rule names its field, the base above the band at 228 m);
  `tesserae-mods` `declare.rs` and `data.rs` (kept, absent, each rule and the
  base through `ModError::Sky`, `biomes.cloud_cover`); `server/sky.rs` (scale 1
  with none declared, the blend along a walk with no step, a scale change alone
  resent); `client/sky.rs` (the scale eases with the pair); `render/clouds.rs`
  (no layer and a cover of 0 draw nothing, which face draws below, inside and
  above, the material from the Sky state, the same material at 1 m and 0.25 m
  Voxels). `clouds.wesl` is read, not compiled, until the first run.

The tests pass in the full `scripts/cargo-ci` run of 2026-11-26.
- Review fix: the empty views from above, inside and overhead were no
  geometry bug (a debug pass showed faces, entry and exit right); the noise was
  too coarse (640 m cells, so a near view saw one cell, often a gap) and bunched
  about 0.5, so a cover of 0.4 showed about 20%. Now 160 m cells, stretched by
  2.5 so the cover is about the share of sky. Inside, the cloud at the Eye
  fogs every ray over 60 m, unfogged by the samples' distance, and the faces
  spread 16 times the reach so no line shows at the horizon. Dusk: clouds are
  lit by the fog colour (0.45) beside the ambient (0.35), and keep 0.6 of
  Daylight of the sun's colour while its light fades at the horizon, so they
  read peach, not brown. Stacked samples still show faint terraced rims at a
  distance; left for the look pass.
