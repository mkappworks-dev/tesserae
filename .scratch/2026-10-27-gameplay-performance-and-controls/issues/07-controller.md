# 07: C3: Controller input, settings and validation

Status: needs-triage
Blocked by: 02, 04, 06

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C3; keep this separate from other PR scopes.

## What to build

Adapt controller input to semantic actions: direct analog move/look in play,
focus navigation in menus. Configure controller bindings, look sensitivity,
invert Y and deadzones independently of keyboard/mouse preferences. Apply
trigger hysteresis, disconnect cancellation and deliberate-input prompt
switching; drift must not switch prompts. No virtual mouse requirement.
Keep desktop touch independent. Validate default mappings on real hardware;
record hardware/OS and distinguish injected tests from physical support.

## Acceptance criteria

- [ ] Complete play/catalogue/assignment/settings without a mouse.
- [ ] Drift causes no movement or prompt churn; look is FPS-independent.
- [ ] Disconnect/focus/menu changes never finish an unwanted hold.
- [ ] Keyboard/mouse preferences and operation survive controller use.
- [ ] Physical hardware validation is required before support claims.

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
