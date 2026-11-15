# 04: Map tiles reach the Client and the minimap draws them

Status: ready-for-agent

Blocked by: 02, 03

Spec: [Phase 19](../spec.md).

## What to build

The Server keeps the declared colour of each Material, from the appearance it
already loads, so a Map tile can be sent with colour while the Map stores only
the Material id. It tracks which Map tiles changed since they were last sent and
sends them as a message under a per-step budget; the Client keeps a bounded cache
and never reads Chunk storage. The Client draws a minimap in the corner, north up
and centred on the Player, at 2 m per pixel in three sizes (128, 168 and 216
pixels), with the Biome name from the list the Server already sends at session
start and the position and height, as in the mock. The minimap is a user
preference, off by default, with a switch and a size choice in Settings under
Display and a key that toggles it in play (`O`). An old profile loads it as off
and Medium. Hidden under covering windows and while dead.

Fog is drawn for what is not Explored, Biome tint and height shading for what is,
and lakes as water.

## Acceptance criteria

- [ ] A Player who walks sees the ground they cross appear on the minimap and nothing they have not seen.
- [ ] Tiles arrive under a per-step budget; a test with many dirty tiles shows the budget holds and all arrive over several steps.
- [ ] The Client cache is bounded and the Client never reads Chunk storage.
- [ ] The minimap is off by default; the setting, the size choice and the `O` key work and persist; an old profile loads as off and Medium.
- [ ] The Biome name and position readout follow the Player.
- [ ] Client tests inject Server tile messages into a minimal app and assert on the UI; fmt, clippy and tests pass.
