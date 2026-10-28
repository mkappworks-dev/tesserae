# 23: S1: Durable World and paused simulation state

Status: done
Blocked by: 22

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: PR 4, step 23 after 22 in one local PR scope.

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

- [x] Evict/reload/restart preserves accepted state and changed coarse terrain.
- [x] Save races/partial writes/failures preserve a recoverable committed save.
- [x] Paused fluid/fire timers resume without offline catch-up.
- [x] Demo reopen does not rebuild/overwrite saved Player edits.
- [x] Save-related frame/tick costs and unsaved/error feedback are measured.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 choices and shared understanding were confirmed. The user
explicitly authorized local PR 4 implementation and focused commits for
tickets 22–23. Push, publication, merge and later scopes remain unauthorized.

## Comments

### Added during grilling

The user requested this scope. Detailed decisions remain open; this ticket
is not implementation-ready. Preserve Server/core/WIT boundaries and the
existing build/profile hygiene. No push, publication or merge is authorized.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.

### Current authorization and Agent Brief

The user confirmed Q1–Q49 and explicitly authorized local PR 4 work for
tickets 22–23 together, superseding the historical pending confirmation
and separate S0/S1 boundaries above. No push, publication or merge. Work
from main b7997b0 in the dedicated persistence worktree; no dependency on
PR 2 changes. Implement durable identity/content, recoverable commits and
Server snapshots before runtime save/eviction/lifecycle integration. Keep
readiness distinct from completion; record actual validation and limits.

### First batch (historical)

The first batch supplied the storage/checkpoint foundation. Runtime and
new/open WIT lifecycle were incomplete then; completion notes below
supersede that intermediate state. See
[save validation](../world-persistence-validation.md) and
[technical contract](../../../../docs/world-persistence.md).

### Completed runtime and validation

Server-owned checkpoints, asynchronous writes, 30-second real-time autosave,
save-before-eviction, paused simulation admission and saved coarse sources
are implemented. Revision/Lifetime guards reject stale completions.
Missing/incompatible content refuses load. New/open WIT lifecycle, exact
Player entry, safe Save & Quit, Retry/Return and confirmed unsafe exit are
covered by regressions. No later-scope controls, World-list, movement,
character, camera or inventory feature was added.

Full cargo-ci passed. Final dev/profiling workspace tests and native
autosave/reopen evidence are recorded in the linked validation document.
Native screenshots remain in /tmp for user review; Windows/Linux runtime
and power-loss behavior are not claimed tested. Measurements characterize
this fixture and do not establish a universal frame/tick bound.
