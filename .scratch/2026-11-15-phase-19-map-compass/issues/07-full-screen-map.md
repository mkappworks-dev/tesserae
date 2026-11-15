# 07: The full-screen Map

Status: done

Blocked by: 04

Spec: [Phase 19](../spec.md).

## What to build

A new action, bound by default to `M` (Cycle tool moved to `O` to make room), opens and closes a Map window in the manner of the Inventory window; it
is also a button in the in-game Menu and is listed in Key bindings, the help text
and the controls documentation. Escape and `M` close it. The window draws the
cached Map tiles with fog of war, Biome tint, height shading and lakes; the
Player's marker is an arrow along their heading; drag pans; zoom steps through 2,
4, 8 and 16 m per pixel by downsampling cached tiles on the Client; a scale bar
shows metres; "Centre on me" returns to the Player; a readout under the pointer
gives position, height, Biome and distance, and says "Not explored" over fog
without naming a Biome; the legend lists the Biomes seen; and the explored area is shown, computed on the Client from the cached tiles as the Explored sample count times 4 m². The window is added to the things that cover the World, so the HUD
hides under it.

## Acceptance criteria

- [x] `M` and Escape open and close the Map; the Menu has a Map button; the action is in Key bindings, help and the controls docs and the shared test that no two default bindings collide still passes.
- [x] Fog, Biome tint, height shading, lakes, the arrow marker, pan, the four zoom levels and the scale bar behave as in the mock.
- [x] The readout over fog says "Not explored" and names no Biome; over explored ground it gives position, height, Biome and distance.
- [x] The window covers the World like other windows (HUD hidden, cursor released).
- [x] The explored area equals the Explored sample count times 4 m² and a test checks it.
- [ ] Client tests drive the window with injected tile messages and key presses; fmt, clippy and tests pass. Tests pass by filter (`map_window`, menu); the full fmt, clippy and test run is deferred to the end of the phase.
