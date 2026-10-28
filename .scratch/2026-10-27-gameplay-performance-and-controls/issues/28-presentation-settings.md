# 28: P5: Display sync, FPS caps and uncapped settings

Status: needs-triage
Blocked by: 03, 04

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: P5; keep this separate from other PR scopes.

## What to build

Expose display-synced normal play by default, configurable FPS caps and an
explicit uncapped option using the user profile/settings lifecycle. Preserve
quality, view distance and simulation rate when changing presentation policy.
Show meaningful selected mode/cap and record actual backend mode in profiling.
Do not infer renderer capacity from a synced/capped FPS counter. Keep
presentation settings separate from optional reduced-quality experiments.

## Acceptance criteria

- [ ] Settings persist/reset and follow Apply/Cancel/unsaved-exit rules.
- [ ] Cap/mode changes never silently alter quality, reach or Server tick rate.
- [ ] Uncapped benchmark mode and normal display-synced play are distinct.
- [ ] Actual mode/focus/target metadata accompanies comparison results.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
