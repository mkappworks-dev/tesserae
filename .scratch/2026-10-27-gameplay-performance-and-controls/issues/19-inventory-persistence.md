# 19: I2: Deferred resource persistence

Status: needs-info
Blocked by: 18, 23

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: I2; keep this separate from other PR scopes.

## What to build

Deferred with owned inventory. Durable World saves alone do not authorize
owned-resource persistence. A future design needs crash-consistent
World/inventory transaction boundaries, stable identities, missing-content
policy and replay/recovery behavior. Do not bundle it into shortcut storage.

## Acceptance criteria

- [ ] Future item policy and transaction/save boundary are separately agreed.
- [ ] Current shortcut/World saves do not claim owned-resource guarantees.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

Owned inventory is explicitly deferred and needs separate future design.

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
