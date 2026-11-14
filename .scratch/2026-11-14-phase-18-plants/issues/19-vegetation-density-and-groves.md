# 19: Vegetation density and groves

Status: done

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

- [x] A headless generation test measures the mean spacing of each feature in a
      sample of Chunks and finds it within 15% of its declared `spacing-m` for
      ungrouped features, at 1.0, 0.5 and 0.25 m.
- [x] A clumped feature has the same mean spacing and a higher variance: groves
      and clearings, measured over the sample.
- [x] Jungle and swamp Biomes are measurably denser than savanna, prairie and
      desert Biomes in every Game that has both.
- [x] A tree across a Chunk border stays whole with a clump.
- [x] The snapshot round trip carries the clump; older saves load as before.
- [x] A Biome with no `clump` generates the recorded digests.
- [x] Generation time with clumps is benchmarked against without.

## Comments

Built. `surface-feature` gains an optional `clump { scale-m, strength }` (WIT in
both copies, the TOML row, `resolve_biomes` with a `BadClump` error naming the
Mod, Biome and feature, core `Clump`, the snapshot's optional `clump`). The
factor is `1 + strength * (2t - 1)`, `t` being value noise (lattice `scale-m`
metres, salted by seed and feature index) steepened by a gain of
`1 + 4 * strength` about 0.5 and clamped to `[0, 1]`; the curve is symmetric, so
the mean is exactly 1 (tested within 5% over 720,000 samples at four settings).
`most` sums each feature at its densest. A strength of 0 generates the same
Voxels as no clump (tested), and every recorded digest still holds.

Tests: `plant_worldgen` (mean spacing within 15% at 1.0, 0.5 and 0.25 m; a
clumped feature has the same mean and over twice the variance of 8 m block
counts, with more empty blocks; the field tracks across Voxel sizes; whole trees
across borders; snapshot round trip and an older snapshot), `mods` (range
errors, resolve, TOML), and `demo_games::vegetation` (trees per hectare per
Biome by name against its class, dense above sparse in all five Games).

Deviations: the spacing lower bound drops from 4 m to 1 m, since the approved
understory is 1 to 2 m; the guest ABI of a 0.3 Declaration changes (the record
gained a field), so guests rebuild. Palms are rare (40 m, clumped) in deserts
rather than "by water only": a feature cannot ask for water nearby. The mockup's
Poisson-disc gap is the engine's mean spacing, so hash-scattered trees can
overlap and the combined tree spacing runs below a headline figure (dense 3.6 to
4.7 m, moderate 5.7 to 8 m, sparse 15 to 67 m). Clump cost is in DESIGN step 18.
