# 02: C0: Semantic actions and defaults

Status: needs-triage
Blocked by: 01

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C0; keep this separate from other PR scopes.

## What to build

Resolve device bindings once per frame into stable semantic ids and
started/held/released/cancelled states plus axes. Convert all current input
consumers, including menus/help/history, to the shared frame. Preserve
existing defaults and modifier alternatives. Specific chords consume their
triggers before simpler actions. Provide extension seams for mode/view/Use,
controller and touch without bundling their adapters.

## Acceptance criteria

- [ ] Default controls retain current Server intent and hold behavior.
- [ ] 60/144/1000-update traces preserve action counts and cancellation.
- [ ] History modifiers do not cause descent or a plain-key action.
- [ ] Simultaneous inputs have deterministic documented priority.

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
