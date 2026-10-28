# 24: M0: Server-owned noclip and physics Player

Status: needs-triage
Blocked by: 02

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: M0; keep this separate from other PR scopes.

## What to build

Evolve the pose speed-limit seam into Server-owned movement/body state.
Support explicit noclip flight and physics walking/sprint/jump/gravity with
Shape-aware core collision, in metres. Camera view does not choose movement
mode. New demo Worlds default physics; persist/restore mode and position via
the save contract when available. Refuse physics entry inside solid terrain,
with feedback instead of teleport. Respect Game/Server mode permissions.
Do not bundle network prediction/transport or read World storage on Client.

## Acceptance criteria

- [ ] Walls, floors, stairs/slabs, ceilings and unloaded terrain are covered.
- [ ] Invalid mode changes cannot embed or teleport a physics body.
- [ ] 60/144/1000-render traces and Voxel sizes preserve movement rules.
- [ ] Hold/toggle sprint and context cancellation remain correct.
- [ ] Save integration restores authoritative Player state.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### Added during grilling

The user requested this scope. Detailed decisions remain open; this ticket
is not implementation-ready. Preserve Server/core/WIT boundaries and the
existing build/profile hygiene. No push, publication or merge is authorized.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
