# 06: Changes under level 0 stop regenerating level 1, if it is worth it

Status: done

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

First a count: level-1 regenerations and meshes per second in `flight`, by cause (a fluid landing, another change, a neighbour arriving, a first generation). If changes under level 0 are 10% of level-1 jobs or less, closed `wontfix` with the figure. Otherwise a level-1 Chunk whose eight children level 0 draws is not regenerated or meshed for any change in them (a fluid's, a Plant's, a Mod's Command, an Edit, an undo) until level 0 stops drawing one of them; then it catches up once, and ADR-0019's rule covers the handover.

Widened on 2026-11-20, with the user, from fluids alone: fluids cause no level-1 work, and the other changes cause most of it.

## Acceptance criteria

- [x] The count in Comments.
- [x] If built: a streaming test that a change under level 0 (a fluid's and an Edit's) queues no level-1 work, and that the level-1 Chunk matches level 0 once one child leaves level 0's ring.
- [x] If built: the holes instrument reports no more holes than 01's figures.

## Comments

### The count, 2026-11-20

Level-1 work by cause (the counter's "edit" is every change but a fluid's: an Edit, a Mod's Command, an undo, a Plant):

| Fixture | Generations: first / fluid / edit / reload | Meshes: first / fluid / edit |
| --- | --- | --- |
| `flight` | 330 / 0 / 676 / 34 | 157 / 0 / 4301 |
| `sprint-flight` | 1650 / 0 / 424 / 139 | 770 / 0 / 2799 |
| `teleport` | 1639 / 0 / 390 / 0 | 564 / 0 / 2381 |

Fluids cause none, so the ticket as written fails its gate. Other changes under level 0 cause 93% of level-1 meshes in `flight` (66% at 64 m/s, 81% after a teleport) and 65% of its generations: the mechanism the ticket describes, from a different cause. Put to the user whether the ticket covers every change under level 0.

One `scripts/profile-frames --fixtures flight sprint-flight teleport --repeats 3` series of dec13212 (the counters on fded14c8's successors: the sister spec's 01 to 05 and this spec's 01, 02, 04 and 07), 11:51 to 12:01, `uptime` load 9.86 falling to 5.52; medians of the three logging-on runs per fixture, counted from the World's opening.

### Built, 2026-11-20

`stream_lod` holds back the mark a change under level 0 makes on its level-1 Chunk while level 0 delivers all eight children: the Chunk goes into level 1's `deferred` instead of `stale`. Each tick `catch_up` marks a deferred Chunk once level 0 stops delivering one child (the Eye moved away or the child was evicted), and the regeneration's landing meshes it. Meanwhile its mesh is unloaded, a mesh in flight for it does not land and none is dispatched, so the Client keeps level 0 drawn over the region until the caught-up mesh arrives (ADR-0019) instead of showing the mesh from before the change; ADR-0011's invalidation paragraph says so. `current_finer` treats a deferred Chunk as stale, so a persistent World's level 2 derives it from the saved sources. Level-1 data is never saved (only level 0 and its revisions are; a reload rebuilds the coarse levels), so nothing stale reaches a save.

Tests in `server::lod::tests`, on rings where level 0 draws all eight children of the Eye's level-1 Chunk and level 1 has no hole:

- `a_change_under_a_cover_level_zero_draws_queues_no_level_one_work`: an Edit, a fluid Batch and a Mod's `Fill` each dispatch no coarse generation or mesh, and the held cover is untouched.
- `a_cover_level_zero_stops_drawing_a_child_of_catches_up_to_the_changed_children`: after the Eye moves four Cells, the cover equals the downsample of the changed children and its new mesh equals one built fresh.
- `no_hole_and_no_stale_cover_shows_while_a_cover_waits_on_level_zero`: a model of the Client's ADR-0019 retention, fed every message from the change through the catch-up, shows each child's region drawn on every tick, and level 1 only with the caught-up mesh. Without the unload it fails.

### Figures, 2026-11-20

The table in [ticket 05](./05-one-light-solve-per-chunk.md#figures-2026-11-20) covers this ticket's columns: against fa6fd574, level-1 meshes fall 38% at 12 m/s, 28% at 64 m/s and 40% after a teleport, and level-1 mesh CPU 33%, 25% and 34%; level-1 generations fall 24% at 12 m/s. Holes are as many as at ticket 01: none in `flight`, 7 frames in each `sprint-flight` run against 7 to 11, 104 to 140 after a teleport against 74 to 156; no rule holes.
