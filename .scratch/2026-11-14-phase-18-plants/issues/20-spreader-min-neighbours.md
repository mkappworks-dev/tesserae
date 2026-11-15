# 20: Spreaders stop creeping: `min-neighbours`

Status: done

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

- [x] A target with fewer than `min-neighbours` of the Plant within reach is
      refused and one with enough is seeded, at 1.0, 0.5 and 0.25 m.
- [x] A row with no `min-neighbours` grows exactly as before.
- [x] A lone seed grows something (a neighbour), bounded: not a field.
- [x] A generated-style patch fills holes and creeps far less than the same
      patch without the rule; the shared ground cover stays within about twice
      its reach over two game days at 1.0 m.
- [x] `min-neighbours` at or above `density` is a load error.
- [x] Grass, tufts, ferns, flowers and mushrooms in `flora` and the Games set
      it.
- [x] Authoring guide, data-mods, contract-compatibility and the ADR-0027
      amendment record the rule and the lone-seed decision.

## Comments

Built. `spreader-def.min-neighbours: option<u32>` (WIT in both copies, the TOML
row, `resolve`, core `Kind::Spreader`), checked at load: below `density` (a
`PlantError::Size` naming the Plant), 0 and 1 no rule. In `reach::spread` the
count is `neighbours`, the sphere count `density` already used, now shared, plus
the seeder when a one-Voxel step in height puts it outside the sphere.

The lone-seed decision. A strict count blocks a lone seed for ever, which is the
case the maintainer named. Two ways out were tried. (1) Require
`min(min-neighbours, c)` where `c` is the seeder's own crowd, so a young colony
asks only for what it sees: a lone seed grew a clump, but a sparse cloud drifted
without end (8 m at 0.5 m Voxels in 6000 Samples), because a target need only be
as crowded as its source and a cloud has plenty of those. Dropped. (2) The chosen
rule: a Voxel with no other of its kind within its own reach is exempt, so a lone
seed (or a lone generated stray) grows one neighbour; a pair is then held to the
full count. A gardener places a handful (a Layout of Seeds) to start a patch.

What it does and does not do. `min-neighbours` and `density` are counts of Voxels
within a reach in metres, so the same row holds more Voxels at a smaller size and
the counts are tuned for 1.0 m, the default. A cluster whose own density sits at
the floor can crawl: for the shared tuft, a worldgen-like
patch of 18 (a third of 8 by 8 m) with the old cap of 3 grew to about 374 over 24
hours reaching the Chunk's edge, and with `density` 5 and `min-neighbours` 4 to
58, 3 Voxels past its square. The 3 m rows (poppy, daisy, dandelion, orchid) need
`density` 9 and `min-neighbours` 7 for the same (57 and 2 Voxels past).
Tests: core `plant::reach` (the count is enforced at 1.0, 0.5 and 0.25 m; none or
1 grows as before; a lone seed grows exactly one neighbour while a free one runs
on; a dense lattice patch fills holes and creeps far less than a free one), core
`plant` (min at or above density refused), mods (resolve and default, load error
naming the Plant, TOML), server (`the_shared_spreaders_keep_a_generated_patch_
where_worldgen_put_it`: 3 and 2 Voxels past against 12 and 12 free; and the
existing hour and 12 hour bounds now run on the real rows).

Decided after the build: `min-neighbours` stays in Voxels and a lone seed grows
exactly one neighbour. A per-metre form, for it and for `density` together, is a
later and separate contract change.

Re-pinned: `the_demo_games_textures_are_unchanged` (the new crops art), nothing
else. No `FORMAT_VERSION` or `GENERATOR_VERSION` change: growth rules are not in
the Biome snapshot.
