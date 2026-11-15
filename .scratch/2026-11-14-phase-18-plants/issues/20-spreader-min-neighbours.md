# 20: Spreaders stop creeping: `min-neighbours`

Status: ready-for-agent

Blocked by: None (the mockup is approved)

Spec: [Phase 18](../spec.md), amendment 2026-11-15.

## What to build

A spreader row gains an optional `min-neighbours` (default 0): a Sample seeds a
target only where at least that many Voxels of the same Plant (Material) stand
within `reach-m` of the target, counted in metres through the existing
`voxels_in` helper in `plant/reach.rs` and counting the seeding Voxel. 0 and 1
are today's behaviour. The existing `density` cap still applies, so
`min-neighbours` must stay below `density` or nothing can ever spread; a row
that sets it at or above `density` is a load error naming the Mod and Plant.

A strict count would block a lone placed seed for ever (it sees only itself), so
a Voxel with no other of its kind within its own `reach-m` is exempt: a lone
seed grows one neighbour, and then, being a pair, is held to the count like
every other Voxel. A generated patch, whose Voxels have company, is held to it
and so fills its holes and its front is slowed.

Contract: `spreader-def.min-neighbours: option<u32>` in the 0.3 WIT (both
copies), the TOML row field, the resolver, the snapshot is unaffected (growth
rules are not frozen into a Biome). Ground cover in `mods/flora` and every
Game's tables gets a value so worldgen patches stay put.

## Acceptance criteria

- [ ] A target with fewer than `min-neighbours` of the Plant within reach is
      refused and one with enough is seeded, at 1.0, 0.5 and 0.25 m.
- [ ] A row with no `min-neighbours` grows exactly as before.
- [ ] A lone seed grows something (a neighbour), bounded: not a field.
- [ ] A generated-style patch fills holes and creeps far less than the same
      patch without the rule; the shared ground cover stays within about twice
      its reach over two game days at 1.0 m.
- [ ] `min-neighbours` at or above `density` is a load error.
- [ ] Grass, tufts, ferns, flowers and mushrooms in `flora` and the Games set
      it.
- [ ] Authoring guide, data-mods, contract-compatibility and the ADR-0027
      amendment record the rule and the lone-seed decision.

## Comments
