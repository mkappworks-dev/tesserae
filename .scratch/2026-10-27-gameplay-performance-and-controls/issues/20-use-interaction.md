# 20: T0: Explicit Use and read-only tool contract

Status: needs-triage
Blocked by: 02

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: T0; keep this separate from other PR scopes.

## What to build

Introduce explicit remappable Use/Interact with Server capability/target
answers and success/refusal/cancellation. Preserve right place/ignite; no
use-before-place fallback. Keep empty hand, sample, physical pickup and use
distinct. Support a read-only measurement mode that disables player place/
remove until exit. Game declares tool content; Engine implements generic
mechanisms. WIT changes follow actual release/version/adapter constraints.

## Acceptance criteria

- [ ] Use never falls back to a surprising Edit on refusal/stale target.
- [ ] Tool mode cancellation restores normal building safely.
- [ ] Mods use bounded WIT, not direct World/core access.
- [ ] No named Game tool or resource consumption is built into Engine.

## Validation and authorization

Use behavior/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures/measurements stay in one unique /tmp folder until review.
All Q1–Q49 product choices are accepted, but final shared understanding is
not yet confirmed. Do not implement, publish, push or merge from this status.

## Comments

### 2026-10-27: repository-based triage

The named decision/evidence gate must be resolved before implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Grilling decision: explicit Use

The user accepted a dedicated remappable Use/Interact action. Right click
remains place/ignite, with no automatic use-before-place fallback. Inventory
ownership is deferred; remaining interaction/tool details still need grilling.
This decision alone does not make the ticket implementation-ready.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
