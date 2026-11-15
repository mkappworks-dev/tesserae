# 05: The minimap

Status: ready-for-agent

Blocked by: 02, 04

Spec: [Phase 19](../spec.md).

## What to build

The Client draws a minimap in the corner, north up and centred on the Player, at
2 m per pixel in three sizes (128, 168 and 216 pixels), from the cached tiles:
fog for what is not Explored, Biome tint and height shading for what is, lakes
as water, the Player's arrow along their heading, and the Biome name (from the
list the Server already sends at session start) with position and height below
it, as in the mock. It is a user preference, off by default, with a switch and a
size choice in Settings under Display (in the "Map and compass" group ticket 02
adds) and a key that toggles it in play (`O`, free in the engine's defaults),
persisted like the Compass preference. An old profile loads it as off and
Medium. Hidden under covering windows and while dead. Waypoints come later
(ticket 09).

## Acceptance criteria

- [ ] A Player who walks sees the ground they cross appear on the minimap and nothing they have not seen.
- [ ] Off by default; the setting, the size choice and the `O` key work and persist; an old profile loads as off and Medium.
- [ ] The Biome name and position follow the Player; hidden under covering windows and while dead.
- [ ] The shared test that no two default bindings collide still passes with `O`.
- [ ] Client tests inject Server tile messages into a minimal app and assert on the UI; fmt, clippy and tests pass.
