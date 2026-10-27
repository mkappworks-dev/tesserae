# 13: P1: Readiness-aware static atlas rendering

Status: needs-triage
Blocked by: 12

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: P1; keep this separate from other PR scopes.

## What to build

Validate the atlas cost using P0, then render only until dirty images,
meshes/materials/pipelines are ready. Cache the completed image; invalidate
on content, textures and item/Shape/orientation set changes. No blind
five-second timeout. Preserve approved appearance, alpha and asset cleanup.
Retain the change only when measured benefit justifies its complexity.

## Acceptance criteria

- [ ] No blank cached icons after shader compile, menu, drag, swap or reload.
- [ ] Clean atlas passes stop and dirty content redraws fully.
- [ ] No image/material/mesh leaks across repeated changes.
- [ ] Equal-quality interleaved data and native visuals support the result.

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
