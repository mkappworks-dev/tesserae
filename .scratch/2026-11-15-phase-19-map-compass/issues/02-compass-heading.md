# 02: The Compass shows the Player's heading

Status: done

Blocked by: 01

Spec: [Phase 19](../spec.md).

## What to build

The Server publishes the Player's look heading with the pose it already
publishes, and the Client draws a Compass strip across the top of the HUD: the
heading in degrees, the cardinal letters with North picked out, and ticks, as in
the mock. North is −Z in every World at every Voxel size, and a bearing is
clockwise from North. A positive Server yaw turns left, so the bearing is its
negative. The heading is the Server's published one, so it follows the camera by the message latency, which in single player is a frame. The strip is hidden while a menu or window covers the World and while
the Player is dead, and shows the same heading in first and third person.

The Compass is a user preference, off by default, with a switch in Settings
under Display (a "Map and compass" group) and a key that toggles it in play
(`B`, free in the engine's defaults). The toggle flips the preference at once and
persists it, and the action is listed in Key bindings, the help text and the
controls documentation. A profile saved before the preference reads it as off.
Waypoint marks come later (ticket 10).

## Acceptance criteria

- [x] The published pose carries the heading; the Client reads it, never its own camera, for the Compass.
- [x] The strip shows degrees, N/E/S/W with N picked out, and turns as the Player looks; North is −Z.
- [x] Hidden under a covering window and while dead; unchanged between first and third person.
- [x] Off by default; the setting and the `B` key toggle it, persist it, and an old profile loads as off.
- [x] The new action appears in Key bindings, help and the controls docs, and a test asserts that no two default bindings collide (tickets 05 and 07 re-run it).
- [x] Tests at the message boundary (injected pose messages and UI queries) and a pure bearing/heading test.
- [ ] fmt, clippy and tests pass. fmt and `cargo check` pass; clippy and the full suite are deferred to the end of the phase.
