# 09: Waypoints: place, name, remove and save

Status: ready-for-agent

Blocked by: 07, 08

Spec: [Phase 19](../spec.md).

## What to build

Waypoints are Server-owned and per Player. The Client sends an add intent
(position, name, Marker kind) and a remove intent; the Server answers each with
a refusal and a reason as it does for other intents. A point must be Explored on
the Player's own Map; a refused add names its reason (fog, no such Marker kind,
a bad name, too many, or removing the death marker). The Waypoint list and the new-Waypoint form in the Map
window work as in the mock: add at the Player's own position or by clicking
explored ground, a default name when none is given, select and centre, remove.
Waypoints draw on the Map and the minimap in their Marker kind's colour and icon,
and a Waypoint beyond the minimap's edge rests on the edge. They save per Player
as a small record in the usual way and survive reopening. A World with no Marker kinds offers none: Add is refused with the reason "no such Marker kind".

## Acceptance criteria

- [ ] Add at the Player's position and by clicking explored ground; clicking fog is refused with a reason.
- [ ] Name, Marker kind, default name, select, centre and remove work; each refusal names its reason, and a Remove of the death marker is refused.
- [ ] Waypoints draw on the Map and minimap with the right colour and icon; one beyond the minimap's edge rests on it.
- [ ] Waypoints are saved per Player and round trip through the persistence rig; a save without them loads with none.
- [ ] A World with no Marker kinds refuses Add with "no such Marker kind" but still shows the Map and minimap.
- [ ] Server tests at the intent seam, Client tests for the form and list; fmt, clippy and tests pass.
