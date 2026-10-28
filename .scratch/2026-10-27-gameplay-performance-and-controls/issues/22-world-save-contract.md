# 22: S0: World identity, save schema and lifecycle

Status: done
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: PR 4, step 22 followed by 23 in one local PR scope.

## What to build

Design the save schema for World identity, seed/parameters/version, stable
content mapping, authoritative solids/Overlay/timers and Player state.
Store names/mappings rather than trusting runtime ids. Distinguish new and
reopened Game lifecycle; reopening cannot replay startup construction over
saved edits. Do not imply a snapshot of the entire WASM process.
Specify versioned commits/recovery and dirty snapshot identity. Apply agreed
30-second/unload/exit saves, offline pause and explicit incompatibility policy.
This contract precedes persistence code, not another unresolved user choice.

## Acceptance criteria

- [x] Schema covers agreed state and explicit recovery/compatibility behavior.
- [x] Same-seed Worlds have distinct durable identity.
- [x] New/open lifecycle prevents demo construction replay on restored data.
- [x] Remaining technical format choices are documented and testable.

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

### Completed contract and lifecycle

Implemented versioned named records, UUID identity, exact compiled-build
requirements, OS writer locking, immutable checksummed commits and explicit
recovery. The amended unreleased 0.3 WIT carries World id and reopening.
The real demo guest builds on new World and emits no construction/scenario
Commands on reopen. Older contracts refuse reopen. Timer/Agenda state,
Player entry and revision/Lifetime guards have automated regressions.
See docs/world-persistence.md and the save-specific validation notes.
