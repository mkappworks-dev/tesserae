# Water motion is drawn, never simulated

Status: accepted.

## Context

Generated water is still: every ocean, lake and river Voxel is a Source, and a
river is a staircase of flat pools held by one-Voxel rock lips. The lips exist
so that no generated water has open air beside it to pour into, because Fluid
that settles in a held Chunk changes it, and a changed Chunk is saved whole
([ADR-0031](0031-a-saved-chunk-is-stored-whole-not-as-a-delta-from-generation.md)).
Water must still look as if it moves: rivers running downstream, a Fall at each
lip, waves on lakes and the sea.

## Decision

**Flow, Falls and waves are presentation.** The Server works out each water
column's Flow and Falls from the World when it builds the Chunk's mesh, from the
Generator's drainage for generated water and from Fluid surfaces for simulated
water, and sends them with the mesh. The Client animates them on its own clock
and reads no World storage ([ADR-0002](0002-client-server-from-day-one.md)).
Nothing of them is stored, and none of them moves a body.

**Generated rivers stay still pools of Sources.** A Fall at a lip is a drawn
sheet and foam, not Fluid.

**Drawn waves stay under 0.15 m**, the underwater look's hysteresis, so the
drawn surface never disagrees with the World's by more than the margin the
underwater test already allows.

## Considered options

- **Spill simulated Fluid at each lip.** Truthful for swimming and buckets, but
  every lip in a held Chunk settles on load and the Chunk is saved whole:
  walking past a river would rewrite it.
- **Waves in `tesserae-core` on a Server clock.** Needed only if waves carry
  bodies or boats. The Server would evaluate them for every body in water, for
  gameplay nothing asks for yet.

## Consequences

- Nothing carries a Player and nothing bobs. Adding either means a wave and Flow
  function in `tesserae-core` on a Server clock, and supersedes this ADR.
- Simulated Fluid, such as water a Player pours or a Settlement's waterfall,
  still flows in the World as before; its Flow is drawn the same way.
- Swimming in a generated river feels like swimming in a lake.
