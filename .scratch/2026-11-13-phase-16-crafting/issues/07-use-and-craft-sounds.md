# 07: Sounds for Use and Craft

Status: ready-for-agent

Blocked by: 02, 03, 05

Spec: [Phase 16](../spec.md).

## What to build

Doors, chests and crafting make sound. Line 0.3's `sound-cause` enum appends
`door-open`, `door-close`, `container-open`, `container-close` and `craft`,
mapped by the Game's `cause-sounds` like any Cause (ADR-0025). The Server emits
each at the Door, the Container or the Player when the action lands, never for a
refusal. Terra ships CC0 sounds for all five, credited.

## Acceptance criteria

- [ ] Each Cause is emitted once per landed action at the right position, and
      not for a refusal.
- [ ] A Game that maps none of them still loads and plays silently.
- [ ] Terra's five sounds are CC0, credited, and play in game.
