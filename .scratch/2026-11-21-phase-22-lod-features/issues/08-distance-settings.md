# 08: The Distance settings tab

Status: ready-for-agent
Blocked by: 07

Spec: [LOD features](../spec.md).

## What to build

A Player sets how far and how sharp distant terrain is from Settings. A Distance
tab after Display, as approved in the mockup: a Preset dropdown (Low, Medium, High,
Custom; Medium the default) with its description, Reach and Detail sliders that only move
under Custom, Custom remembering its own values, and a table of each level's
Voxel size, ring, Chunk count and estimated memory that follows the sliders.
The table and the sliders' ends use the World's own view distance and ceiling,
not the mockup's 160 m and 8 km. Detail is shown in whole Chunks and sent as metres (`u` = Chunks ×
32). Low is the Game's own rings: its view distance, and the reach it declared before this step. 07's
`QualityCurve::balanced` and `detailed` and the `TESSERAE_QUALITY` names become `low`, `medium` and
`high` (spec Q24). Applying sends the curve to the Server (07) and
keeps it in the Player's profile.

## Acceptance criteria

- [ ] Settings tests for the dropdown, the locked sliders, Custom's memory,
      apply, discard and restore defaults.
- [ ] Applying a preset changes the rings the Server streams.
- [ ] A capture of the tab, shown to you beside the mockup.
