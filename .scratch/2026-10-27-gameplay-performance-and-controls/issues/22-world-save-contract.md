# 22: S0: World identity, save schema and lifecycle

Status: needs-triage
Blocked by: none

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: S0; keep this separate from other PR scopes.

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

- [ ] Schema covers agreed state and explicit recovery/compatibility behavior.
- [ ] Same-seed Worlds have distinct durable identity.
- [ ] New/open lifecycle prevents demo construction replay on restored data.
- [ ] Remaining technical format choices are documented and testable.

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
