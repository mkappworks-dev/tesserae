# 06: C2: Search, readable labels and responsive catalogue

Status: needs-triage
Blocked by: 05

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C2; keep this separate from other PR scopes.

## What to build

Search Material/Shape names with All/Solids/Fluids/Fire and Mod filters.
Remember browsing within the current World session; reset on another World.
Use short tile names, full identity in details/tooltips and visible duplicate
name disambiguation. Add UI/label sizing while preserving default appearance.
Keep footer fixed with horizontal scrolling where needed, not clipped slots.
Validate 1024x640 at 100%, and 1280x720/1920x1080 at 100/150/200% UI scale.
Touch targets are at least 44 logical pixels with spacing.

## Acceptance criteria

- [ ] Text input never moves, respawns, edits or activates gameplay shortcuts.
- [ ] All required controls/slots remain reachable in the size/scale matrix.
- [ ] Long, duplicate, missing and empty content remain readable.
- [ ] Browsing restores within a session and resets on another World.
- [ ] Native review covers normal/small/enlarged layouts.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### 2026-10-27: repository-based triage

Draft scope/acceptance require grilling; do not begin implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
