# Connections are derived from neighbours, never stored

A fence grows arms towards the fences and walls beside it. Whatever holds that
fact has to be right after an Edit, after a Chunk generates next door, after a
save and a load, and after a Mod changes what connects to what.

**The Voxel's bytes do not change.** `material:16 | shape:8 | orientation:8`
stands as [ADR-0006](0006-shapes-are-table-data.md) left it, and a connection is
computed where it is needed from the six Voxels around: by the mesher from the
`Neighborhood` shell a job was handed, by the sweep and the ray through
`VoxelVolume`. `ShapeTable::connections` is the one rule all three ask.

Four ways to hold it were weighed:

- **Derived at read time.** Chosen.
- **Stored in the orientation byte.** It would fit, since a connected Shape has
  no use for orientation. It would also make every Edit rewrite up to six
  neighbours, some in another Chunk; leave a fence generated beside an existing
  one with no way to learn it should grow an arm; and keep a fact that is a
  function of other facts, which a Mod changing `connects_to` leaves stale in
  every saved fence. Each fails only after a save or at a seam.
- **A 64-bit Voxel.** Doubles every palette entry for two Shapes' worth of
  benefit, and is the save migration ADR-0006 exists to avoid.
- **One ShapeId per pattern.** Sixteen of the 256 ids per fence, plus the
  neighbour-rewriting Edit of the second option.

## What it costs, and what pays for it

Two rules, checked at registration, keep the derivation off the hot paths:

- **A connected Shape seals no face under any arms**, so pass 1's occlusion
  test, the Chunk fast paths and both light solvers stay free of neighbour
  context. A fence blocks no light and hides no face. A Shape that would seal is
  a cube with extra steps.
- **A connected Shape has no Orientation.** Arms are indexed by world face and a
  Voxel's orientation byte reads as the identity, so registration derives `2^k`
  variants for `k` arms instead of `24 x 2^k`.

A third makes culling exact with one neighbour: an arm touches only its own
face's boundary plane, so whether a neighbour's arm covers ours depends on us
and nothing else.

The cost accepted is that the mesher, the sweep and the ray read up to six
neighbours for a connected Voxel, which they already hold, and only a connected
Voxel pays. Pass 2 culls the end of an arm against a neighbour's arm exactly;
pass 1 culls against a neighbour that seals, so a cube's face under a fence's
rails is drawn: hidden overdraw, never a hole.

## Consequences

A Mod declares connections as data in the one batched `register` call (groups,
`connects_to`, whether to meet what seals, and the arms). A callback for "does
this connect?" is excluded by [ADR-0001](0001-wasm-mod-boundary.md): it is a
host call per neighbour per Voxel. The `ShapeTable` is fixed once the World is
open ([ADR-0007](0007-streaming-jobs-run-on-bevy-task-pools.md)) and a job reads
connections only from its snapshot.

Groups belong to Shapes, not Materials, until step 7 gives Materials a registry.

Collision may stand one Voxel outside its cell (`CollisionBox`, signed), which
is how a fence is 1.5 Voxels tall to bump into and one to look at. The table
records the overhang per face and the sweep widens its range by it. Picking
tests the drawn boxes (`selection`), not collision, so the part of a fence only
a body feels is not something a player can aim at. All of it is a fraction of a
Voxel and becomes a distance by `voxel_size`
([ADR-0003](0003-voxel-size-is-per-world.md)).

A connected Shape costs the Chunk's shaped cap in stairs: its quads at its most
armed over a stair's, rounded up.
