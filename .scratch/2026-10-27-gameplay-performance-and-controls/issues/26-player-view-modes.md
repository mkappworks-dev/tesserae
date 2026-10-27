# 26: V0: Independent first/third-person camera and aim

Status: needs-triage
Blocked by: 24, 25

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: V0; keep this separate from other PR scopes.

## What to build

Support remappable first/rear-third switching independently of movement.
Restore saved view; new demo Worlds default first-person. Third-person is a
centred rear chase camera with zoom and wall avoidance, no selfie/cinematic
mode. Keep ordinary wheel as slot selection and Alt+wheel as remappable
third-person Zoom In/Out defaults.
Crosshair picks a visible point but Server validates Player-Eye reach and
occlusion. Invalid points show feedback, not fallback edits. Camera offset
cannot move Player or grant reach. Obstacle queries preserve no-Client-storage.

## Acceptance criteria

- [ ] Both views work in flight/physics without moving the Player on toggle.
- [ ] Near-wall/narrow-space/zoom transitions avoid camera terrain clipping.
- [ ] Crosshair/ghost/cues align and blocked targets never edit around corners.
- [ ] Hotbar wheel and modifier zoom cannot trigger competing actions.
- [ ] View restore and all device semantic actions are covered.

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
