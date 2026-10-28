# 18: I1: Deferred owned inventory policy

Status: needs-info
Blocked by: 17

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: I1; keep this separate from other PR scopes.

## What to build

Deferred by the accepted creative-first scope. Finite Item identity,
stack/consume/durability/drop and survival undo policy are not approved.
Do not implement Server item transactions until those policies are separately
grilled. Any later design must preserve atomic World/resource operations,
idempotent per-Player requests and exact rejection behavior.

## Acceptance criteria

- [ ] Separate future approval establishes item/mode/undo policy first.
- [ ] No finite inventory behavior enters the current creative work.

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
