Status: ready-for-agent

# Phase 17: Terrain and water

DESIGN.md roadmap step 17, Generator version 3, in four parts: 17a Landforms and
oceans, 17b Water, 17c Underground, 17d Biomes. Depends on step 13 (Biomes,
done). Weather (step 24) reads the Climate this step produces; LOD features
(step 22) read its fields. The Engine adds the generator; a Game declares tuning
and Biomes.

Vocabulary: **Landform**, **Sea level** and **Water surface** are new and
**Climate**, **Biome** and **Water level** are amended in
[GLOSSARY.md](../../GLOSSARY.md). The decision record is
[ADR-0030](../../docs/adr/0030-generator-version-3-is-a-layered-field-stack-in-a-declared-band.md),
with an amendment to
[ADR-0022](../../docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md).
Related: [ADR-0003](../../docs/adr/0003-voxel-size-is-per-world.md),
[ADR-0011](../../docs/adr/0011-coarse-levels-are-derived.md),
[ADR-0013](../../docs/adr/0013-fluid-and-fire-live-beside-the-voxel.md),
[ADR-0015](../../docs/adr/0015-water-is-game-content.md),
[ADR-0016](../../docs/adr/0016-only-air-is-built-in.md),
[ADR-0020](../../docs/adr/0020-data-only-mods-and-development-restarts.md) and
[ADR-0029](../../docs/adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md).

## Problem Statement

Terrain is one height noise, one flat lake level and one tunnel field. There are
no oceans, rivers, mountains with character, caverns or Biomes that depend on
where the land is. A Biome never changes the ground, so a desert is flat and a
plain has no river a bridge could cross. Every World looks the same at every
scale.

## Solution

A layered generator, all of it a pure function of the seed and a position in
metres, so it holds at every Voxel size and in any Chunk order:

- **17a.** A fixed stack of 2D fields (continentalness, erosion, peaks and
  valleys, derived ruggedness), domain-warped with a clamped displacement and
  mapped by curves to a base height inside a declared vertical band. A bounded
  3D density near the surface, zero on flat ground, gives overhangs, arches and
  sheer cliffs. Oceans have a shelf, slope, deep floor and trenches.
- **17b.** One Water surface function per column: the sea at Sea level, lakes
  at their spill height, rivers stepping down their course with waterfalls
  where they drop. Drainage is solved per region on a jittered lattice with a
  fixed halo and cached.
- **17c.** Strata, caverns, tunnels, ravines, underground water, lava near the
  floor, aquifers and cave Biomes chosen in three dimensions.
- **17d.** Climate gains altitude and coast terms; Biomes gain altitude, coast
  and erosion selectors and a relief scale for their landform's detail. The Games
  declare the new Biomes and water.

Versions 1 and 2 keep their terrain and recorded digests. Version 3 becomes the
default and is accepted by the save only when all four parts have landed.

## User Stories

1. As a Player, I see oceans, coasts, plains, hills, plateaus, canyons and
   mountain ranges, not one rolling noise.
2. As a Player, I walk a river from a mountain to the sea, and it ends in a lake
   or the sea, never in the middle of the ground.
3. As a Player, I find caverns, ravines and underground lakes, and cave Biomes
   that differ from the surface above them.
4. As a Game author, I tune scales, the band, Sea level and each landform's
   strength in metres, and I declare Biomes with altitude, coast and erosion
   ranges.
5. As a Game author, a Biome I declared before this step still works unchanged.
6. As a Player with a World made under version 1 or 2, my World is exactly as it
   was.
7. As a Mod author, I never place terrain; the Engine places what is declared.
8. As a Player at 0.5 m or 0.25 m Voxels, the same coast, river and cave are in
   the same place in metres.
9. As a Player looking at distant land, the coarse levels agree with the near
   ones about coast, river and mountain.
10. As the Server, I still have one global bound on the surface for the vertical
    streaming band and open-sky relight.
11. As a Settlement or a bridge, I see rivers and the sea through the same Water
    surface function.
12. As Weather (step 24), I read one Climate function of position and altitude.
13. As a Player spawning, I start on dry land clear of cliffs.

## Implementation Decisions

All numbers below are placeholders to tune against screenshots and benchmarks;
the spec records the shape, not the constants.

**Versioning and snapshot.**

- `GENERATOR_VERSION` moves to 3 and `SUPPORTED_GENERATORS` gains 3 in the last
  ticket only. Until then a World cannot be created or saved under 3, but tests
  generate under it by passing the version to `Worldgen`.
- Terrain height, caves and the water consumers dispatch on the version through
  one function, replacing the single `BIOMES_SINCE` comparison. Versions 1 and 2
  call today's code unchanged.
- Every new parameter is an optional field of the save's `Generation` snapshot,
  defaulted when absent, so an older save reads as before.

**The band.** `floor_m` and `ceiling_m` in metres, plus `overhang_m`. The global
surface bound is `[floor_m - overhang_m, ceiling_m + overhang_m]` widened by
Plants and Structures as today. The chunk early-outs, `above_the_surface` and
the Structure band read it. A mountain past the ceiling is clamped. Uniform deep
rock sits below the deepest underground feature, which is part of the band.

**Fields.** Continentalness, erosion and peaks-and-valleys are 2D fields of
metres (continent about 6 km across), warped by a field whose displacement is
clamped so the band holds. Ruggedness is derived. Curves map them to base height.
The new primitives live in `noise.rs` beside the old ones, which stay: ridged
noise, a warp helper and a 2D range bound for early-outs.

**Overhangs.** Within `overhang_m` of the base surface the density is
`(base - y) / scale + noise3 * amp`, with `amp` zero where ruggedness is low.
Levels 1 and 2 get it from their downsample; levels 3 and up generate columns
and read the heightfield, as they drop caves (ADR-0011).

**Water surface.** `water_surface_m(x_m, z_m) -> Option<f32>`: the sea at Sea
level, a lake at its spill height, a river stepped by its course. Settlements,
crossings, the Waterline, LOD level 1, spawn and the Map ask it and nothing
else. The flat `water.level_m` stays for versions 1 and 2. A World with no Game
fluid has the basins, dry.

**Drainage.** Regions of a placeholder 2 km, each with a jittered lattice hashed
from the seed and the region. A priority-flood pass finds basins and their spill
heights; flow accumulation gives river segments with a width that grows
downstream. A fixed halo of neighbouring regions (placeholder three) is part of
the window, so the result is a pure function of the seed and the window. A river
that would need more upstream than the halo is cut at it. Regions are cached in
a bounded LRU on the generator; a miss recomputes, never reads a Chunk. The
height function subtracts a valley profile by distance to the nearest segment;
the Water surface steps down it, with a waterfall where the drop exceeds a
Voxel.

**Underground.** Strata are layers by depth below the surface with their
Materials declared per Biome like ground. Caverns are a thresholded low-frequency
3D field gated by depth; tunnels are the existing two-field Cave retuned for 3;
ravines a ridged slot stretched vertically. Lava lakes sit near the floor.
Aquifers and underground water come from the water table. Cave Biomes are
selected by 3D fields from a list declared like surface Biomes, capped at 16.
Every carve is gated by the band and never breaches the sea floor or sealed
surface unintentionally.

**Climate and Biomes.** `climate_at(x, y, z)` returns temperature, which falls
with altitude by a lapse rate, and humidity, which rises toward the sea. Biomes
gain `altitude`, `coast` and `erosion` ranges and a `relief` scale (and
roughness) that multiplies the detail octaves of their landform only. A Biome
with no range matches every value on that axis, so every earlier declaration
stays valid. Borders blend relief and cave tuning smoothly; ground still
switches hard. Ocean, shore and river Biomes are ordinary declared Biomes.

**Contract.** Everything declared is appended to the unreleased 0.3 Declaration
as optional fields (`docs/contract-compatibility.md`); no new WIT line.

**Consumers.** Surface features, Plants, crop fields, Structures, Settlements
and crossings read the new height and Water surface through the existing
functions and keep their rules. A steep slope refuses ground cover and
Structures through the existing ground rule. Spawn picks the nearest dry land at
least a metre above the Water surface and clear of cliffs.

## Testing Decisions

- Recorded digests for versions 1 and 2 (`tests/generator_version.rs`,
  `surface_features.rs`, `settlement_worldgen.rs`, `coarse_generation.rs`) pass
  unchanged.
- Version 3 digests are recorded in the last ticket, at 1.0 and 0.5 m.
- Property tests: any Chunk order gives the same Voxels; a coast, river and cave
  are at the same metres at every Voxel size; coarse levels agree with level 0
  on the coastline and valley to within a coarse Voxel.
- Drainage tests: every river reaches a lake or the sea; no basin lacks a spill;
  region borders and the halo cut agree from both sides; the cache changes no
  result.
- Bound tests: no Voxel is solid above the global bound or the early-out
  declares uniform where it is not.
- Benchmarks, interleaved CPU-time A/B against `main` under the host-load rules
  (`worldgen/straddling`, early-outs, a river Chunk, a cavern Chunk, a coarse
  level-3 Chunk): surface Chunks within about 3x of version 2 and the 3.8 ns
  early-outs unregressed. A screenshot set, offscreen, of each landform, a river
  and a cave, kept in one `/tmp` folder and deleted at review.

## Out of Scope

- Weather, rain and snow (step 24), which read this Climate and never edit
  terrain.
- Trees and Structures at coarse levels (step 22).
- Erosion by time, moving water beyond the existing fluid simulation, tides.
- Dimensions and a second World layer.
- A Game-declared landform kind or field expression.
- A global erosion simulation or a precomputed heightmap.

## Further Notes

- Research: the layered-field and spline approach, domain warping, a
  per-region drainage solve with a halo, and band-bounded 3D density; rivers
  cannot come from local noise alone. Only a few sources were fetched, so cite
  the papers when an implementer adopts one.
- The first step of every ticket is a failing test, per `/tdd`.
- Tickets are in [issues/](./issues/).
