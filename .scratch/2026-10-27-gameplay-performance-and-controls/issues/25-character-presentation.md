# 25: A0: Basic Game-provided blocky character

Status: needs-triage
Blocked by: 24

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: A0; keep this separate from other PR scopes.

## What to build

Add a simple Game-provided blocky humanoid and idle/move/sprint/jump states.
Engine owns presentation mechanisms, not named character content. Follow
Server body state; the rendered avatar is not movement authority.
Show full body in third-person, hide in first-person and retain existing
held model. No character creator, skins/cosmetics or new first-person arms
system. Rebuild guests/demo after Game/WIT changes and review visuals.

## Acceptance criteria

- [ ] Art/content ownership and bounded declaration/asset loading are explicit.
- [ ] Animations follow movement/ground/air states without authority drift.
- [ ] First-person has no duplicate held model or clipping body.
- [ ] Native appearance/animation review and performance evidence are recorded.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### Added during grilling

The user requested this scope. Detailed decisions remain open; this ticket
is not implementation-ready. Preserve Server/core/WIT boundaries and the
existing build/profile hygiene. No push, publication or merge is authorized.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
