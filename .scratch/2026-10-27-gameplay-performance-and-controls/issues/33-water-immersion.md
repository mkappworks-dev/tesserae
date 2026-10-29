# 33: Water immersion: view and movement

Status: needs-triage
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: its own PR; keep this separate from other PR scopes.

## What to build

Raised by the user on 2026-10-29 while testing ticket 27's demo lakes.

- **View:** with the camera inside water, the user sees no water, only the
  void or the surrounding ground, in both flight and physics modes. Water
  surfaces are drawn from outside only and there is no underwater tint or
  fog. Draw the surface from below and add an underwater view.
- **Movement:** in physics mode water has no effect on the Player: no
  buoyancy, drag, swimming or sinking. The spec's physics movement (ticket
  24) never included it.

Movement must stay Server-owned and computed in metres (ADR-0002, ADR-0003);
the view is Client presentation. Needs grilling before implementation.
