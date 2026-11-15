# 10: The Compass shows Waypoints, and a death leaves one

Status: done

Blocked by: 02, 09

Spec: [Phase 19](../spec.md).

## What to build

The Compass strip shows each Waypoint at its bearing in its Marker kind's colour
and icon, one off to the side rests at the edge of the strip, dimmed, and the
name and distance of the Waypoint the Player is facing show beneath the heading,
as in the mock. The Server adds a Waypoint where a Player died, of the Marker
kind the Game names for death (a Game that names none gets no death marker); a
new death replaces the previous death marker, the Player cannot remove it by
accident (the list has no remove control on it) and a death marker is saved like
any Waypoint.

## Acceptance criteria

- [x] Waypoints appear on the Compass at the right bearing as the Player turns; an off-side one rests dimmed at the edge.
- [x] Facing a Waypoint within a few degrees shows its name and distance.
- [x] Dying adds one death Waypoint of the Game's death kind at the death position; a second death replaces it; no death kind means none.
- [x] The death marker survives save and reopen and has no remove control in the list.
- [ ] Tests at the Server death hook and the Client Compass seam; fmt, clippy and tests pass. (Tests written and passing by filter, fmt run; clippy and the full suite are deferred to the end of the phase.)
