# 01: C0: Explicit input ownership

Status: needs-triage
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C0; keep this separate from other PR scopes.

## What to build

Own input in gameplay, catalogue, settings capture, text entry and unfocused
contexts. Resolve same-frame UI/context changes before gameplay consumers.
Movement, look, respawn, place/remove, history and sampling obey that owner.
Cancel active axes, partial actions and pending samples across context/focus/
device changes. Require fresh destructive activation on return. The Server
continues simulating while UI is open. Do not mutate raw mouse state to
simulate other devices or introduce Client World access.

## Acceptance criteria

- [ ] UI focus prevents gameplay movement, respawn, Edits, history and samples.
- [ ] Capture/close/regain-focus inputs never leak a destructive activation.
- [ ] Held input cannot resume cancelled work; pending samples are discarded.
- [ ] Exercise actual input consumers and same-frame ordering in regressions.

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
