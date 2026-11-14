# 18: Branching trees

Status: done

Blocked by: None (the mockup is approved first)

Spec: [Phase 18](../spec.md), amendment 2026-11-14.

## What to build

Trees are not a straight trunk under a ball. A `tree` row may declare how its
trunk and limbs branch, and the species in `flora` use it, so an oak forks and
spreads, a pine has tiers of drooping branches, a palm leans, an acacia forks
into a flat crown and a jungle tree buttresses and spreads.

- `tree-def` gains an optional `branching`: how many branches and from what
  fraction of the trunk height they start, their reach, rise or droop and
  thickness, whether the trunk forks into leaders, and how far the whole trunk
  leans, all in metres or fractions of the tree's height so the tree is the same
  at every Voxel size. Canopy foliage grows as lobes around the branch tips and
  the leaders as well as around the crown, sized by `canopy-radius-m`.
- The shape stays a pure function of the row, the seed, the base cell and the
  Voxel size, so a tree is whole across Chunk borders and generated and grown
  trees are identical. Its horizontal reach, lean and lobes together stay within
  the existing 8 m bound, so worldgen's scan bound does not grow.
- Branches are trunk Voxels of the tree's grown Material. Felling needs no new
  rule, but its region size bounds are recomputed from the declared reach. A
  branch is cut by Remove like any trunk Voxel and what hung on it falls.
- Fruit hangs from branch-tip lobes.
- The approved mockup shows each species' branching.

## Acceptance criteria

- [x] An oak, birch, pine, palm, acacia, jungle and swamp tree each have the
      shape the mockup shows, from a headless generation test that checks
      branch Voxels exist at the declared heights and reaches.
- [x] A branching tree is whole across Chunk borders and equal in metres at
      1.0, 0.5 and 0.25 m, grown and generated alike.
- [x] Felling a branched tree breaks the part above the cut, branches and lobes
      included, and drops one sapling for the tree; a cut branch leaves the
      trunk standing.
- [x] A tree with no `branching` generates exactly as before.
- [x] The landing and generation cost of a branching tree is benchmarked and
      `TREES_PER_STEP` rechecked.
- [x] ADR-0027 and the authoring guide describe `branching`.

## Comments

Mockup drawn at `/vegetation` (sections "Branching trees" and "Density and
spacing", Biome matrix density tags), awaiting the maintainer's approval. The
`branching` declaration it shows: fork height, leader count, limb heights,
directions and reach, lobe size, all in metres. Spacing per Biome type is also
in metres. Code is in `mockup/branching.js` and `mockup/density.js`.

Built. `tree-def` gains an optional `branching` (limbs, start, reach-m, rise,
fork, lean-m, tiers, lobe-radius-m), resolved and validated in
`tesserae-core` `Branching::check` (the lean, spread, limb and lobe together
stay within 8 m, so worldgen's scan bound does not grow; a fork's spread stays
inside the crown and lobes so the tree is one piece) and frozen into the saved
generation data as an optional field. Limbs are 6-connected stepped lines of
the trunk Material, lobes ellipsoids of the canopy Material at each tip and the
crown; the shared `flora` species use it. Tests: `plant::tree` (shape per
species, pieces, lean, fork, tiers, equality in metres at 1.0/0.5/0.25 m,
felling a trunk and a limb, a row without `branching` generating exactly as
before against a copy of the old generator), `plant_worldgen` (whole across
Chunk borders, caves and smoothing, snapshot), the Farm rig (grows whole,
all-or-nothing, relight) and two Server felling tests. Numbers are in ADR-0027
Consequences and DESIGN step 18: the oak lands in 10.2 ms at 0.25 m against 6.1
for the plain one, so `TREES_PER_STEP` stays 1.

Deviations: limbs are one Voxel thick at every size (no 2-Voxel limbs), the
jungle tree has no buttresses and no limb bends or drooping tips (the mockup's
hand-drawn details), and a cut limb drops a sapling like any region (once), as
a cut canopy already did. A lobe touching another part's lobe keeps a cut part
up, since Felling joins grown Voxels through faces.
