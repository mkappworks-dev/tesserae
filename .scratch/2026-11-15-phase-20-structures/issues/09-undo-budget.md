# 09: One undo removes a whole Structure

Status: ready-for-agent

Blocked by: 08

Spec: [Phase 20](../spec.md).

## What to build

A placement is one undo group. The journal counts cells, 131,072 in all, with
its 128 groups, instead of 1024 entries. Undo and redo stay all or nothing with
their present checks.

## Acceptance criteria

- [ ] One undo removes a whole Structure and one redo restores it.
- [ ] A placement fills the budget by evicting the oldest groups, never the
      wrong ones, and a Template at the cap fits twice.
- [ ] Single-cell Edits are as undoable as before, by the existing journal
      tests.
- [ ] Undo is refused as it is now when a Container the Structure placed has
      since been filled.
