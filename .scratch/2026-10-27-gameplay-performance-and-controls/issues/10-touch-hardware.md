# 10: C4: Real desktop touch acceptance

Status: needs-triage
Blocked by: 09

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C4; keep this separate from other PR scopes.

## What to build

Run two-finger move/look, controls, holds, catalogue browse/assignment,
interruptions, focus loss and mouse return on actual supported-desktop touch
hardware. Record OS/device/scaling/event delivery and synthetic mouse
behavior. Mobile emulation or this Mac without touch hardware is not a
substitute. Record visual approval independently from physical validation.

## Acceptance criteria

- [ ] Actual contacts and synthetic mouse behavior match injected regressions.
- [ ] No accidental/duplicate Edit across focus or gesture interruption.
- [ ] Controls remain reachable/readable at the specified logical sizes.
- [ ] Untested platforms/hardware remain explicitly unclaimed.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### 2026-10-27: repository-based triage

Completion requires real desktop touch hardware and physical input evidence.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
