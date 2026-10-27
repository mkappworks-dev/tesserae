# 21: T1: Read-only corner measurement

Status: needs-triage
Blocked by: 20, 06

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: T1; keep this separate from other PR scopes.

## What to build

Game supplies the first tool: read-only measurement. Use picks two Server-
resolved Voxel/Shape corners within normal Edit reach. Show endpoint preview,
line, straight-line and X/Y/Z differences in metres and Voxel units. Movement
between endpoints permits a larger span. Keep one completed measurement for
the World session; replace on a new measure and provide Clear/Cancel.
Disable player place/remove while measuring. No saved marker collection,
free-point drawing, mining or container transactions in this PR.

## Acceptance criteria

- [ ] Shape/orientation/Voxel-size corner results are correct in metres.
- [ ] Stale, blocked or out-of-reach endpoint selection is refused clearly.
- [ ] Measuring never edits World or consumes resources.
- [ ] Clear/replace/cancel/session reset and device input paths work.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### 2026-10-27: repository-based triage

The named decision/evidence gate must be resolved before implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
