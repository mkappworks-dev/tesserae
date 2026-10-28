# 17: I0: Creative organization and per-Game shortcuts

Status: needs-triage
Blocked by: 03, 06

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: I0; keep this separate from other PR scopes.

## What to build

Keep nine quick slots and temporary outside-held selection. Add explicit
Clear/Swap; clearing removes a shortcut, X empties only the hand and invalid
drag drops cancel. Save arrangements per Game using stable content/Shape
names and Orientation; missing content is visibly empty. Do not save the
temporary held selection or introduce bag pages, quantities or resources.
Keep read-only sampling and current creative placement Server paths.

## Acceptance criteria

- [ ] X/sample/outside selection preserve all assigned shortcuts.
- [ ] Clear/swap/select/return journeys work with each supported input path.
- [ ] Restart/content-order/missing-content cases never alias runtime ids.
- [ ] No stacks, consumption, drops or physical pickup are introduced.

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

### Grilling decision: creative organization first

The user selected unlimited creative selection/quick-slot organization for
the first inventory PR. Finite owned stacks, consumption and physical pickup
remain later work. Organization/persistence choices are in the next round.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.
