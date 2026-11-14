# 19: Vegetation density and groves

Status: ready-for-agent

Blocked by: None (the mockup is approved first)

Spec: [Phase 18](../spec.md), amendment 2026-11-14.

## What to build

Plants are scattered at distances that suit their Biome. A jungle packs trees a
few metres apart under a dense understory, a temperate forest is moderate with
clearings, and a savanna, prairie, desert or frost Biome is sparse. Today every
feature is an even hash scatter, so a Biome reads as a regular grid and a sparse
Biome can only be made sparse everywhere.

- **Content tuning.** Every Biome of every Game gets feature spacings in metres
  that match its density class in the approved mockup: dense (trees about 3 to 5
  m, understory 1 to 2 m), moderate (7 to 10 m), sparse (20 to 40 m), with ground
  cover finer than trees. Spacing is in metres, so the density is the same at
  every Voxel size.
- **Groves and clearings.** A feature may declare a `clump`: a low-frequency
  density field of position in metres (a scale and a strength from 0 to 1) that
  multiplies its chance, so trees gather in groves with clearings between and
  ground cover grows in patches. It is a pure function of the seed and a column
  position, so it holds across Chunk borders and at every Voxel size. A feature
  with no `clump` scatters as today.
- The chance of a column stays `(voxel size / spacing)^2` before the clump, and
  the clump's mean over a Biome is 1, so the declared spacing is still the mean
  spacing.
- The clump is part of the frozen feature in the World's Biome snapshot (an
  optional field), and a Biome with no clump generates exactly as before.

## Acceptance criteria

- [ ] A headless generation test measures the mean spacing of each feature in a
      sample of Chunks and finds it within 15% of its declared `spacing-m` for
      ungrouped features, at 1.0, 0.5 and 0.25 m.
- [ ] A clumped feature has the same mean spacing and a higher variance: groves
      and clearings, measured over the sample.
- [ ] Jungle and swamp Biomes are measurably denser than savanna, prairie and
      desert Biomes in every Game that has both.
- [ ] A tree across a Chunk border stays whole with a clump.
- [ ] The snapshot round trip carries the clump; older saves load as before.
- [ ] A Biome with no `clump` generates the recorded digests.
- [ ] Generation time with clumps is benchmarked against without.
