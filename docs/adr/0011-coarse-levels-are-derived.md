# Coarse LOD levels are derived, never stored and never edited

Status: accepted.

## Context

A Chunk is 32³ at every LOD level, so a level N+1 Chunk covers eight of level N.
The Server holds level 0 only within `evict` of the Eye (288 m at the defaults),
so the finer Chunks a distant coarse one would be made from do not exist. There
is also no save yet, so nothing durable says what an Edit changed.

## Decision

**A Chunk of level 1 or above is a pure function of the seed, the parameters and
the level-0 Chunks the Server holds that an Edit changed.** It is not save data
and no Edit is ever applied to it.
`Worldgen::new(seed, voxel_size * 2^j, params)` is the level-j source generator,
because worldgen is tuned in metres. A level-k Chunk is the downsample of eight
level-(k-1) Chunks generated for the purpose; level 1 takes the held level-0
Chunks instead of generating them.

**Fluid and fire at level 1.** A held Chunk something changed is read with its
fluid and fire (`downsample_changed` in `lod.rs`), a picture of that state and
never a simulation of it. Fluid weighs the volume it fills and competes with
solids by volume as generated water does. Water the generator could have made, a
packed source below its waterline, stays packed lake water; a coarse Voxel whose
fluid is mostly any other is air with that fluid beside it, standing at the
share of the coarse Voxel its contents fill and at least a sixteenth. A coarse
solid burns when half its Material's volume does, and is drawn with flames.

The `ChunkStore`, light, relight and `MeshState` stay level 0 only. Coarse
Chunks live in a Server-private `LodStore`, so
[ADR-0002](0002-client-server-from-day-one.md) holds as it did for `ChunkStore`:
the Server is the only writer and the Client receives meshes and nothing else.
Jobs follow [ADR-0007](0007-streaming-jobs-run-on-bevy-task-pools.md).

**Invalidation.** An Edit marks its level-1 cover stale, if it is held. So does
every other change through the one World change, a Mod Command or landed flow,
fire or reaction, including one that leaves the packed Voxel as it was. A held
level-0 Chunk that has been edited marks its cover stale when it leaves the
store, because the Edit is about to stop existing. An unedited Chunk leaves with
no mark: the generator reproduces it exactly, and a test pins that. Levels 2 and
above read the generator alone and are never invalidated; an Edit is metres from
the Eye and level 2 starts at 256 m.

## Consequences

- **An Edit is visible at levels 0 and 1 only**, and only while its level-0
  Chunk is held. Both are already true of level 0, which has no save. Fluid and
  fire are the same: from level 2 out, and at level 1 once their level-0 Chunk
  is evicted, the water is where the generator put it, so a breached lake is
  whole again, a waterfall and a placed pool are gone and nothing burns. The
  step is at level 2's inner edge, 256 m at the defaults, inside the 288 m that
  level 0 is held to.
- **Emitters vanish at distance.** A coarse Voxel is its dominant Material and
  carries no block light. Nothing generates an emitter today. Lava and fire at
  level 1 are drawn and emit nothing; there is no coarse emitter flood, and
  fluid beside a coarse Voxel is lit as the air it stands in.
- **Only the lake meets level 0's waterline.** The mesh job moves a coarse lake
  surface to the top of level 0's highest water Voxel, so a Chunk boundary
  cannot fall between the two. Fluid that moved is never moved: its level-0
  surface is wherever it flowed, and its coarse surface stays in the coarse
  Voxel holding it, which lies in the level-0 Chunk holding that water. Where
  four level-0 columns of one coarse column end on both sides of a level-0
  Chunk's floor, one coarse surface cannot sit on both.
- **Coarse light is two-valued.** A level above 0 stores no light. The mesh job
  lights a face open sky unless it looks into air more than one Voxel under
  `Worldgen::surface_height_m`, which it lights dark. The rule is `coarse_light`
  in `lod.rs`, a column rule from the generator and not a flood, which is why
  `light.rs` takes no new rule. What it gets wrong: a pit open to the sky is
  dark below its top Voxel where level 0 lights it, a sideways opening is dark
  from its first covered Voxel where level 0 grades over about 15 m, and the
  dark value does not depend on depth. A capture of the default seed's mouth at
  250 m found the difference a few dozen pixels, so the rule is small on
  purpose. See `.scratch/2026-10-14-lod-coarse-light/spec.md`.
- **The save format is unchanged.** It has been LOD-aware since step 1. A later
  save step stores level 0 only and rebuilds the rest, as this does.
- **Thin features vanish at the next level.** A one-Voxel pane is under half a
  coarse Voxel; this is what a mip does.

## Revisit when a save step makes level 0 durable

The coarse levels then read it instead of the generator and need no other
change, which lifts the first loss above. Placed lanterns being durable is also
when "a coarse Voxel holding an emitter keeps its Material if it passes the
half-volume test" becomes worth its one line.
