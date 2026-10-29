# 17: I0: Creative shortcuts and per-Game arrangements

Status: done

Spec: [creative organization](../spec.md#creative-organization-and-interaction).
Implementation: merged PR #24, main `4b86e43`.

## Completed scope

Use nine quick slots plus a temporary outside-held selection. Catalogue clicks
and read-only sampling can choose content without overwriting assigned slots.
X and Clear selection empty only the hand. Existing assignment and drag/drop
replace shortcuts; dragging one slot onto another swaps them. Invalid outside
and cancelled drops leave the arrangement intact.

Persist the nine slots and active slot per Game using stable Material/Shape
names and Orientation. Missing content resolves to empty, never to unrelated
content at a reused runtime id. Temporary outside-held selection is not saved.
These are creative shortcuts, separate from global controls and World saves.

## Acceptance criteria

- [x] Exactly nine quick slots and temporary outside selection are supported.
- [x] X, Clear selection, sampling and outside selection preserve assigned
      slots.
- [x] Existing assignment, drag-swap and slot-selection journeys work.
- [x] Invalid or cancelled outside drops do not delete shortcuts.
- [x] Restart, reordered ids and missing content retain stable content identity
      and Orientation per Game.
- [x] Outside-held selection remains transient.
- [x] Read-only sampling and Server placement retain their existing paths.
- [x] No stacks, consumption, pickup or owned inventory is introduced.

## Completion decision

The user accepted the existing main behavior as complete and removed the
additional organization UI from scope. This narrowed scope is authoritative;
no new slot-operation controls remain pending in this ticket. Do not restore
the reverted PR 3 UI as follow-up work.

PR #25 separately hardens failed hotbar writes so they can retry without a new
layout edit. That fix does not change the accepted menu workflow or format.
Existing restart/name/Orientation/Game-isolation and write-failure regressions
are recorded in [final validation](../pr3-implementation-validation.md).
