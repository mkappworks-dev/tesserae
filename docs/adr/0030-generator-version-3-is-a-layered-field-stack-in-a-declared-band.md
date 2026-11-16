# Generator version 3 is a layered field stack inside a declared band

Status: accepted.

Step 17 reworks worldgen from one height noise, one flat lake level and one cave
field into landforms, oceans, rivers, a layered underground and Biomes chosen by
altitude and coast. A World made under version 1 or 2 keeps its terrain and its
recorded digests
([ADR-0022](0022-biomes-are-column-content-and-a-world-pins-its-generator.md)).

## Decision

**A declared vertical band.** A World declares a floor and a ceiling in metres.
Every land and sea-floor field is clamped inside it, and the 3D overhang term is
bounded by a declared reach, so the band plus that reach is the one global bound
that the Chunk early-outs, the Server's open-sky relight and the Structure band
read. A peak above the ceiling is clamped, never refused. `sea_level_m` stops
meaning the lowest ground: a true Sea level sits inside the band.

**A fixed stack of fields.** Continentalness, erosion, peaks and valleys and a
derived ruggedness, all pure functions of the seed and a position in metres,
domain-warped with a clamped displacement, each mapped by a curve to a base
height. The Engine owns the Landform algorithms; a Game declares tuning in
metres and cannot add a kind, because a declarable kind would need an expression
language across the WIT contract ([ADR-0001](0001-wasm-mod-boundary.md)).

**Overhangs near the surface only.** A 3D density within the declared reach of
the base surface, zero where ruggedness is low, so plains and sea floors are
heightfields. LOD levels 3 and up generate columns and drop it, as
[ADR-0011](0011-coarse-levels-are-derived.md) already drops Caves there.

**One Water surface.** Settlements, bridges, the Waterline, coarse levels and
spawn ask one function per column, never a World-wide level, so a river a bridge
must cross is a river every consumer sees.

**Drainage per region.** The World is cut into regions, each with a jittered
lattice hashed from the seed; a drainage solve with a fixed halo of neighbours
finds lake basins, flow and river segments, cached per region. The result is a
pure function of the seed and a fixed window, so any Chunk order agrees. A river
that would need more upstream than the halo is cut at it, a cost accepted.

**Delivery.** Parts 17a-d land together; the version number moves to 3 and the
save accepts it only when they all have, so a half-built layout is never pinned.

## Rejected

- Biomes that move the base surface (ADR-0022's reason stands).
- A global erosion simulation: bounded-domain, not seed-pure on an infinite
  World.
- An erosion-look filter alone: no real rivers or lakes, so no drainage for
  bridges and Weather to read.
