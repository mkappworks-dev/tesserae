# 23: S1: Durable World and paused simulation state

Status: needs-triage
Blocked by: 22

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: S1; keep this separate from other PR scopes.

## What to build

Persist Player/Mod/simulation changes across eviction, return and reopen,
including removals and fluid/fire state/timers. Autosave every 30 seconds,
save before changed Chunk eviction and on Save & Quit. Use safe worker
snapshots and dirty-version/Lifetime guards; preserve the last completed
recoverable save. Never falsely mark failed/newer edits saved.
Resume paused unloaded/offline state without catch-up. Coarse derivation
must read saved changes, not overwrite them with generation. Refuse missing
required content/incompatible loads without Air substitution or save writes.
Failed Save & Quit keeps World open with Retry/Return; confirm unsafe exit.

## Acceptance criteria

- [ ] Evict/reload/restart preserves accepted state and changed coarse terrain.
- [ ] Save races/partial writes/failures preserve a recoverable committed save.
- [ ] Paused fluid/fire timers resume without offline catch-up.
- [ ] Demo reopen does not rebuild/overwrite saved Player edits.
- [ ] Save-related frame/tick costs and unsaved/error feedback are measured.

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
