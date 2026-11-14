# 18: Branching trees

Status: ready-for-agent

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

- [ ] An oak, birch, pine, palm, acacia, jungle and swamp tree each have the
      shape the mockup shows, from a headless generation test that checks
      branch Voxels exist at the declared heights and reaches.
- [ ] A branching tree is whole across Chunk borders and equal in metres at
      1.0, 0.5 and 0.25 m, grown and generated alike.
- [ ] Felling a branched tree breaks the part above the cut, branches and lobes
      included, and drops one sapling for the tree; a cut branch leaves the
      trunk standing.
- [ ] A tree with no `branching` generates exactly as before.
- [ ] The landing and generation cost of a branching tree is benchmarked and
      `TREES_PER_STEP` rechecked.
- [ ] ADR-0027 and the authoring guide describe `branching`.
