# 08: The Distance settings tab

Status: ready-for-agent
Blocked by: 07

Spec: [LOD features](../spec.md).

## What to build

A Player sets how far and how sharp distant terrain is from Settings. A Distance
tab after Display, as approved in the mockup: a Preset dropdown (Balanced,
Detailed, Custom) with its description, Reach and Detail sliders that only move
under Custom, Custom remembering its own values, and a table of each level's
Voxel size, ring, Chunk count and estimated memory that follows the sliders.
The table and the sliders' ends use the World's own view distance and ceiling,
not the mockup's 192 m and 8 km. Applying sends the curve to the Server (07) and
keeps it in the Player's profile.

## Acceptance criteria

- [ ] Settings tests for the dropdown, the locked sliders, Custom's memory,
      apply, discard and restore defaults.
- [ ] Applying a preset changes the rings the Server streams.
- [ ] A capture of the tab, shown to you beside the mockup.
