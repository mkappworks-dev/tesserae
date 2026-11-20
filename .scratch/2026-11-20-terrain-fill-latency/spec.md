Status: ready-for-agent

# Terrain fill latency: the ground near the Eye drawn sooner

Its sister spec, [Generation cost](../2026-11-20-generation-cost/spec.md), makes each Chunk cheaper to generate; this one makes generated terrain reach the screen sooner, on the same branch, judged by the same flight runs. Background reading: [ADR-0002](../../docs/adr/0002-client-server-from-day-one.md), [ADR-0007](../../docs/adr/0007-streaming-jobs-run-on-bevy-task-pools.md), [ADR-0011](../../docs/adr/0011-coarse-levels-are-derived.md), [ADR-0019](../../docs/adr/0019-a-level-is-drawn-until-another-replaces-it.md).

## Problem Statement

Two things a Player sees in Terra:

- **Spawning into a new World**, the loading screen lifts, the Voxels near the Player are missing for a while, and then everything appears at once. The screen lifts once any terrain is drawn, and the coarser LOD levels, cheap and dispatched coarsest first, can arrive before level 0. Then each level-0 mesh waits for the light around its Chunk to settle, so the near ground appears in a burst. A teleport, which refills the view the same way, takes 2.55 s for level 0 to settle; it took 0.78 s on 2026-11-01, before Generator version 3.
- **Flying**, at up to 64 m/s, the ground, trees and Structures near the Eye take a couple of seconds to appear, and meanwhile LOD level 1 covers them, bare, without trees or Structures.

Where the workers' time goes, ms of CPU per second (from the [sister spec's figures](../2026-11-20-generation-cost/spec.md#further-notes)):

| Fixture         | L0 relight | L0 solves per Chunk | L1 mesh | L1 meshes per Chunk | All stages |
| --------------- | ---------- | ------------------- | ------- | ------------------- | ---------- |
| `flight`        | 90         | 7.3                 | 272     | 4.4                 | ~0.9 s     |
| `sprint-flight` | 404        | 7.5                 | 155     | 1.6                 | ~2.3 s     |
| `teleport`      | 89         | 4.2                 | 130     | 1.3                 | –          |

- Every stage of every level, and the fluids, share one first-in-first-out pool of three workers. Up to 64 generations, 128 light solves and 128 meshes of level 0 and 48 LOD jobs can be queued ahead of a near Chunk. Those caps were sized when a surface Chunk cost 252 µs to generate; a version 3 one costs milliseconds.
- A level-0 Chunk is lit about seven times per generation, and a level-1 Chunk meshed about four times at 12 m/s. Those ratios are measured; their causes are read from the code, not counted: a solve against absent neighbours repeated as each neighbour's face plane moves, a level-1 mesh repeated as neighbours arrive, and fluid landings may drive both. Tickets 05 and 06 count them by cause first.
- A level-1 mesh job computes the whole coarse light before the mesher finds the mesh is empty, so a Chunk of air inside the terrain band pays a full light pass for nothing.
- A Chunk that cannot have a face (all air, or rock walled in by rock) still waits for its neighbours' light and a trip through the pool before its empty mesh is sent.
- A level-1 generation job whose level-0 children are not yet held generates them itself, and level 0 generates them again where they lie inside its load extent; how often is not yet counted.
- Every fluid change in a held level-0 Chunk regenerates the level-1 Chunk over it, and if that changes its mesh, meshes it and its six neighbours, though level 0 draws over all of them.
- Nothing measures how late terrain appears near the Eye: the holes count only Cells no level draws, and a level-0 Cell covered by a coarser one is not a hole.

## Solution

- An instrument measures how far from the Eye level 0 is missing, and how long a Chunk takes from entering the mesh extent to being drawn.
- Work that cannot change what is drawn is skipped: light for an empty level-1 mesh, a level-1 Chunk's own generation of level-0 children level 0 is about to generate, light and a pool trip for a Chunk with no faces, and level-1 work under level 0 caused by fluids.
- A light solve waits until the neighbours it reads are held, so each Chunk is solved about once.
- The loading screen lifts when the ground around the Eye is drawn at full detail.
- Last, gated on a 64 m/s flight and a teleport: if level 0 still falls behind while workers have spare time, the streaming caps or the queue order bind, and a sweep, then a priority queue if the sweep is not enough, fixes them.

## User Stories

1. As a Player opening a new World, the loading screen lifts once the ground around me is drawn at full detail, not at the first distant Chunk, and a World with no Game still opens after the timeout.
2. As a Player after a spawn or a teleport, the ground around me fills in sooner and never later than it does now.
3. As a Player flying over Terra, the ground, trees and Structures near me are drawn at full detail sooner, so the land around me is not drawn bare by a coarser LOD level while I fly.
4. As a Player flying at 64 m/s, level 0 keeps up with me as well as the machine allows, and where it cannot, the figures say why.
5. As a Player, nothing I see changes but how soon: the same Voxels, the same light, the same meshes once settled, no new holes.
6. As a Player beside moving water, the coarser LOD levels still match what level 0 shows once I fly away.
7. As the Server, no Client system mutates World state, and `tesserae-core` gains no Bevy or wgpu.
8. As a contributor, a profiling instrument reports per frame the distance from the Eye to the nearest Cell in view that level 0 should draw and does not, and per Chunk the time from entering the mesh extent to being drawn, by distance band.
9. As a contributor, each ticket records `flight`, `sprint-flight` and `teleport` against the starting figures, three interleaved rounds, with the instrument, worker CPU and job counts per level and stage.
10. As a contributor, a change the measure does not justify is closed with its figure.
11. As a reviewer, each change is one commit with its tests and figures.

## Implementation Decisions

**Order.** The instrument and the starting figures, then empty level-1 meshes skipping light, a verified win, and the count that gates level 1 waiting for level 0's children, interleaved with the sister spec as its Order says; then the rest by measured share, the loading screen, and last the cap and queue gate.

**The instrument.** Beside the holes count, off unless asked for: per frame, the distance from the Eye to the nearest Cell in the frustum that level 0's ring holds and level 0 does not draw, whatever coarser level covers it; per level-0 Chunk, the time from entering the mesh extent to its mesh being drawn, as percentiles per 32 m distance band. The Server's streaming stats record when a Chunk enters the mesh extent, the Client records when its mesh is drawn from what it was sent, and the profiler joins the two, as it already reads the streaming stats; no Client system reads or mutates World state (ADR-0002). `scripts/profile-frames` summarises both per fixture. Targets for the end: at 12 m/s level 0 is never missing within 96 m of the Eye; at 64 m/s the median is at least 64 m; a teleport's level 0 settles in under 1 s.

**Empty level-1 meshes.** The mesher's test that a Chunk emits no faces, whatever its light, is public in `tesserae-core`, and `mesh_coarse` asks it before computing coarse light.

**Level 1 waits for level 0, gated.** First a count: level-0 children a level-1 job generates though they lie inside level 0's load extent. At 10% of level-1 generation or less, closed `wontfix`. Otherwise a level-1 Cell all eight of whose level-0 children lie inside level 0's load extent is not dispatched until level 0 holds them; its job then reads them as held Chunks instead of generating them. Any other level-1 Cell is dispatched as now. Near Cells that level 1 would have covered after a teleport stay empty until level 0 arrives, so the holes instrument is the proof that no hole opens.

**Faceless Chunks.** A level-0 Chunk that cannot have a face whatever the light (the two cases `light_does_not_revive_either_declined_case` covers) is sent its empty mesh on the tick it lands, without a mesh job or the settled-light wait.

**One light solve per Chunk, gated.** First the solves are counted by cause: a fill (a neighbour arriving) or a change (fluids, Edits). Runs if level 0's relight is more than 10% of worker CPU in `sprint-flight` after the generation tickets. A light solve is not dispatched until its four sideways neighbours and the one above are held or outside the load extent, and solves are ordered top-down within distance bands rather than across the whole load extent. Bulk light only increases, so a later solve converges to the same light.

**Fluids under level 0, gated.** First a count: level-1 regenerations and meshes per second in the flight, by cause (a fluid landing, a neighbour arriving, a first generation). Above 10% of level-1 jobs, a level-1 Chunk whose eight children level 0 draws is not regenerated or meshed for a fluid change until level 0 stops drawing one of them; ADR-0019's rule covers the handover.

**The loading screen.** It hides once the Client holds a level-0 mesh for every Cell of the mesh ring around the Eye, the Client's own count of what it was sent, not on the first terrain drawn; the 15 s timeout stays. That is the whole ring, so the screen lasts about as long as a teleport's settle, 2.55 s at the start, until the other tickets bring it down. A UI change: the mockup shows the stage text first and is approved before the Client implements it.

**Caps and the queue, gated.** After every other ticket of both specs, the targets above are measured. If level 0 misses them while the workers have spare time, a sweep of `TESSERAE_BUDGET`, `TESSERAE_LOD_BUDGET` and `TESSERAE_SETTLE_GATE` picks new defaults, recorded in the doc comments of `StreamingBudget` and `LodBudget` as their earlier sweeps are. If no setting meets them, the pool's jobs are taken from a priority queue the tick re-scores when the anchor moves, instead of in arrival order. If the workers are saturated, generation binds: closed with the figures, and the sister spec's level-2 option is put to a human.

## Testing Decisions

- Streaming tests in the style of the existing ones (`hold`, `scramble`, the in-flight caps): a faceless Chunk's empty mesh lands on its tick; a light solve is not dispatched while a sideways neighbour inside the load extent is missing, and the settled light equals the light today's order gives; a level-1 Cell inside level 0's load extent is not dispatched before its children are held, and its Coarse equals one generated alone.
- `mesh_coarse` of a Chunk that emits no faces equals today's result, and computes no light.
- The loading screen: a Client test that it stays up while a level-0 Cell of the mesh ring has no mesh, hides when every one has, and hides at the timeout.
- No hole: the holes instrument over `flight`, `sprint-flight` and `teleport` reports no more fill or rule holes than the starting figures.
- Figures: `scripts/profile-frames --fixtures flight sprint-flight teleport`, three interleaved rounds, against the starting build, with `uptime` recorded and nothing else building.

## Out of Scope

- The GPU cost of drawing Chunks: shadows, culling and placeholders are the render thread's (`tech/render-shadows-and-culling`).
- Generation cost, which is the sister spec's.
- Trees, Structures or crop fields on LOD level 1 and beyond, and the level-0 distance Terra declares (160 m): land past it is drawn bare by design.
- LOD level 2 drawn from direct columns, unless the last gate puts it to a human.

## Further Notes

- The Client's mesh upload is not what binds: 32 a tick at 11 to 16 µs is about 0.5 ms, and meshes are built on the Client's own pool.
- Per-tick scans proportional to every held Chunk (`retire`, `SentBelow::new`, `sort_blended`) cost microseconds; not taken.
- The first step of every ticket is a failing test, per `/tdd`.
- Tickets are in [issues/](./issues/).
