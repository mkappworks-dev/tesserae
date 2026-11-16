# 11: Biome selectors and relief (17d)

Status: ready-for-agent

Blocked by: 10

Spec: [Phase 17](../spec.md).

## What to build

Altitude, coast and erosion ranges and a relief scale on Biomes, in the unreleased 0.3 Declaration and the TOML loader, snapshotted in the save. A Biome naming no range matches all. Strata too (moved from ticket 08): rock layers by depth below the surface, their Materials declared per Biome like ground.

## Acceptance criteria

- [ ] Every earlier Biome declaration loads and behaves as before.
- [ ] Relief multiplies detail octaves only; the band and base surface do not move.
- [ ] Borders blend relief and cave tuning smoothly.
- [ ] A bad range is a load error naming the Biome.
