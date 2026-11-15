# 09: Waypoints: place, name, remove and save

Status: done

Blocked by: 07, 08

Spec: [Phase 19](../spec.md).

## What to build

Waypoints are Server-owned and per Player. The Client sends an add intent
(position, name, Marker kind), an edit intent (name, Marker kind) and a remove intent; the Server answers each with
a refusal and a reason as it does for other intents. A point must be Explored on
the Player's own Map; a refused add names its reason (fog, no such Marker kind,
a bad name, too many, or editing or removing the death marker). The Waypoint list and the popover in the Map
window work as in the mock: the Add Waypoint button turns on add mode, then a click on explored ground opens the
add popover there, or the At my position link adds one at the Player's own position and opens its edit popover;
a default name when none is given; a click on a mark or a list row opens the edit popover (rename, change kind, remove),
panning only when the mark is out of view; the death marker's popover is read-only.
Waypoints draw on the Map and the minimap in their Marker kind's colour and icon,
and a Waypoint beyond the minimap's edge rests on the edge. They save per Player
as a small record in the usual way and survive reopening. A World with no Marker kinds offers none: Add is refused with the reason "no such Marker kind".

## Acceptance criteria

- [x] Add at the Player's position and by clicking explored ground; clicking fog is refused with a reason.
- [x] Name, Marker kind, default name, edit, select and remove work; each refusal names its reason, and a Remove of the death marker is refused.
- [x] Waypoints draw on the Map and minimap with the right colour and icon; one beyond the minimap's edge rests on it.
- [x] Waypoints are saved per Player and round trip through the persistence rig; a save without them loads with none.
- [x] A World with no Marker kinds refuses Add with "no such Marker kind" but still shows the Map and minimap.
- [x] Server tests at the intent seam (add, edit, remove), Client tests for add mode, the popover and the list.
- [ ] fmt, clippy and tests pass (clippy and the full suite are deferred to the end of the phase).
