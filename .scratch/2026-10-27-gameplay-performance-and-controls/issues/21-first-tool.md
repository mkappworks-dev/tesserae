# 21: T1: Read-only corner measurement

Status: done
Blocked by: 20 (same PR; 06 done)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: one PR with ticket 20 (sequence position 8). Contract first,
then the demo tool.

## What to build

The demo Game declares the first tool, "Measure" (`kind = measure`), a
read-only measurement on the contract of ticket 20. The Engine hardcodes no
tool name.

### Why this tool first

It needs no inventory, durability, consumption or World mutation, so it
proves Use, the declaration path and the Server's aim answer without
reopening tickets 18-19.

### Selecting endpoints

- Use picks two endpoints. With the tool held this is left click / RT (Remove)
  by ticket 20; right click / LT (Place) clears the last point.
  The Server resolves each as a corner in metres plus its Voxel coordinates;
  the snapped corner is the vertex of the struck face (Orientation- and
  nodebox-aware) nearest the ray's hit point, not of the whole Voxel bounds,
  so a stair's inner step corner is selectable and a gap between two walls
  can be measured from the faces that bound it.
- Measure aims at solid Voxels only, by the solid ray that Place uses: it
  passes flowing fluid and fire and stops at solids. Fluid sources and fire
  are not endpoints. With no solid in reach the refusal is "nothing to
  measure".
- Each endpoint must be within normal Edit reach of the Eye when selected.
  The Player may move between selections to measure a larger span; the
  completed span is not reach-limited.
- A stale, blocked, unloaded or out-of-reach selection is refused with the
  Alert toast of ticket 20. A refusal keeps any endpoint already placed.
- A preview shows the snapped corner before Use, from `use_target`.

### State

- The Server stays stateless. The Client holds up to two endpoints as
  presentation state and computes the readout from Server-supplied numbers.
- A third Use starts a new measurement, replacing the completed one. One
  completed measurement is kept for the World session.
- `ClearTool` (default `Backspace`) drops the in-progress endpoint, then,
  pressed again, the completed measurement. Selecting another hand keeps the
  endpoints, shown only while the tool is held. A World switch clears them.
  Endpoints are never persisted. Simulation keeps running while measuring.

### Presentation

- Endpoint markers, the connecting line and the three axis-aligned legs
  between the endpoints (X, Y, Z, each coloured) are world-space gizmos,
  visible in both views and not hidden behind the avatar.
- The snap preview is tinted invalid before the press when out of reach.
- A corner HUD panel, shown only while the tool is held and an endpoint
  exists, in the quiet-UI card style: straight-line distance, X/Y/Z
  differences in metres, and X/Y/Z differences in Voxel units (metres over
  the World's `voxel_size`, derived, never the source). A hint line reads the
  current bindings, for example "G to place point, Backspace to clear".
- First person shows a small content-only held model through the existing
  held-model path. Third person adds no pose (a new arms
  system stays deferred).

### Not included

Saved marker collections, free-point drawing, mining, container
transactions, resource consumption, touch adapters.

## Acceptance criteria

- [x] A stair's inner step corner can be selected, and a gap between two
      walls can be measured from their facing faces (slab, post and stair are
      tested at 1.0, 0.5 and 0.25 m; Orientation rides on the shared rotated
      boxes and is tested for the identity turn only).
- [x] Corners are correct in metres for stairs, slabs and posts at every
      Orientation, at Voxel sizes 1.0, 0.5 and 0.25 m; the Voxel-unit column
      agrees.
- [x] Stale, blocked, unloaded or out-of-reach endpoint selection is refused
      clearly and keeps prior endpoints.
- [x] Two endpoints taken after walking farther than one reach apart work;
      each is individually reach-checked.
- [x] Endpoints from first and third person agree for the same target.
- [x] Measuring never edits the World, journals, or consumes resources
      (state hash and journal length unchanged).
- [x] Replace, `ClearTool`, hand change, World-switch reset and rebinding all
      behave as above on keyboard and pad paths.
- [x] A Game declaring no tool offers no Measure: `CycleTool` does nothing.
      Measure is taken with `CycleTool` (`M`), not from the Catalogue.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review;
the user reviews the markers, line and HUD visually. Physical controller use
is validated by the user, not by injected tests. All Q1-Q49 product choices
and the 2026-10-30 grilling (Q1-Q19) are accepted and shared understanding is
confirmed. Local implementation is authorized; push, publication and merge
are not.

## Comments

### 2026-10-27: repository-based triage

The named decision/evidence gate must be resolved before implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above.

### 2026-10-30: grilled and triaged

The user accepted recommendations Q1-Q19 together with ticket 20. Ticket 06
is done. Ships in the same PR as ticket 20. Status is ready-for-agent.

### 2026-10-30: design review against similar games

Applied: face-vertex snapping, coloured axis legs, tool clicks as the main
input, invalid preview tint. Deferred: per-tool reach (see ticket 20), a
volume/size readout and a metres/Voxel unit toggle.

### 2026-10-30: implementation complete

Implemented with ticket 20 (see its comment). Corners snap to the nearest
vertex of any box in the struck face's plane, so a stair's inner step corner is
selectable. The readout (distance, X/Y/Z in metres and Voxels), the coloured
axis legs, the endpoint markers and the snap preview are Client presentation
from Server-resolved corners. Left unreviewed natively: how the markers, legs
and readout panel look, which the user checks from a capture. Deferred as
recorded: a volume readout, a unit toggle and a per-tool reach.
