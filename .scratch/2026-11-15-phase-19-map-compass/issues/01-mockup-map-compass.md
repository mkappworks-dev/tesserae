# 01: Mockup: the Map, Compass and minimap

Status: done

Blocked by: None (can start immediately)

Spec: [Phase 19](../spec.md).

## What to build

The Client's look for the Map and the Compass lands in
[`mockup/`](../../../mockup/README.md) first and is approved before any Client
work. The step 19 section shows the full-screen Map (fog of war, Biome tint,
height shading, zoom 2 to 16 m per pixel, pan, hover readout, scale bar, legend,
Waypoint list and new-Waypoint form), the Compass strip with Waypoint marks and
the facing label, the minimap, and the Display settings and key bindings.

## Acceptance criteria

- [x] The mockup shows every surface in the spec's Client section, in the existing visual language.
- [x] `python3 mockup/server.py` serves it at `/map` and in the Gameplay HUD.
- [x] The maintainer approved it on 2026-11-15 ("lgtm"); the Map key is `M` (Cycle tool moved to `O`), and the Compass is off by default with `B` and `N` toggling the Compass and minimap.
- [x] `npm run fmt:check` passes.
