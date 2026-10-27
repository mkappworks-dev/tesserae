# 16: P4: Evidence-gated bounded emitter relight

Status: needs-info
Blocked by: 12

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: P4; keep this separate from other PR scopes.

## What to build

Measure emitter relight using existing 64 Hz CPU/wall fixtures, including
0.25 m lava tails. Design a bounded equivalent algorithm before code.
Preserve Occupants/Lifetime, settled light, wake/Edit/LOD invalidation and
current simulation/Mod count bounds. A rule/bound change requires its own
review, not an FPS label. Preserve interleaved thread-CPU methodology.

## Acceptance criteria

- [ ] Concrete algorithm and equivalence proof precede implementation.
- [ ] Equal events give equal results across sizes/worker counts.
- [ ] Player Edits are not lost or deferred behind bulk work.
- [ ] Report logical backlog/visible latency separately from render FPS.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

This optimization remains evidence-gated; no algorithm is pre-approved.

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
