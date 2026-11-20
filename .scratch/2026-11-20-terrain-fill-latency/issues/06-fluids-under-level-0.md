# 06: Changes under level 0 stop regenerating level 1, if it is worth it

Status: ready-for-agent

Blocked by: 01

Spec: [Terrain fill latency](../spec.md).

## What to build

First a count: level-1 regenerations and meshes per second in `flight`, by cause (a fluid landing, another change, a neighbour arriving, a first generation). If changes under level 0 are 10% of level-1 jobs or less, closed `wontfix` with the figure. Otherwise a level-1 Chunk whose eight children level 0 draws is not regenerated or meshed for any change in them (a fluid's, a Plant's, a Mod's Command, an Edit, an undo) until level 0 stops drawing one of them; then it catches up once, and ADR-0019's rule covers the handover.

Widened on 2026-11-20, with the user, from fluids alone: fluids cause no level-1 work, and the other changes cause most of it.

## Acceptance criteria

- [x] The count in Comments.
- [ ] If built: a streaming test that a change under level 0 (a fluid's and an Edit's) queues no level-1 work, and that the level-1 Chunk matches level 0 once one child leaves level 0's ring.
- [ ] If built: the holes instrument reports no more holes than 01's figures.

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
