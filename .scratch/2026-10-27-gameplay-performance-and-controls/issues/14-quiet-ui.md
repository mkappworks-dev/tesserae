# 14: P2: Measured quiet UI work

Status: needs-triage
Blocked by: 12

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: P2; keep this separate from other PR scopes.

## What to build

Measure hidden diagnostics, inactive progress/place cues and stable preview
collection. Reduce proven recurring work with run conditions/change detection
or readable display throttling. Preserve profiling collection independently
of HUD visibility. Do not throttle live aim/progress feedback or miss content
updates. Do not add complex caches without system/layout evidence.

## Acceptance criteria

- [ ] First show has fresh panel state; quiet traces avoid pointless churn.
- [ ] Progress/target timing remains correct.
- [ ] Catalogue/artwork updates are never missed.
- [ ] Interleaved equal-quality timing reports actual benefit and tails.

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
