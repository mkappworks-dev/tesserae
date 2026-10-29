# 06: C2: Material search and readable catalogue

Status: done

Spec: [catalogue and readable UI](../spec.md#catalogue-and-readable-ui).
Implementation: merged PR #24, main `4b86e43`.

## Completed scope

Use the existing Material-name search and approved menu presentation. Search
text is retained when switching menu pages. Tiles show short Material names;
the detail panel shows the full Material identity and named Shape choices.
Cards wrap to the available width. Materials scroll above a fixed quick-slot
footer, whose slot area supports horizontal scrolling when needed.

## Acceptance criteria

- [x] Search filters declared Materials without changing their runtime ids.
- [x] Search text never activates movement, respawn, edit or other gameplay
      shortcuts.
- [x] The search editor retains text and focus while filtering results.
- [x] Empty results show a readable message and leave the quick slots available.
- [x] Tiles use short names and details expose the full Material identity.
- [x] Wrapping cards and a fixed, scrollable slot footer preserve the approved
      main layout.

## Completion decision

The user accepted this existing behavior as complete and requested removal of
unimplemented additions from the ticket. The main menu layout and settings
remain the accepted presentation. This narrowed scope is authoritative; there
are no further search, filter, sizing or browsing-lifecycle tasks in ticket 06.
Do not reintroduce the reverted PR 3 UI as pending work.

Existing search/editor/input-ownership and catalogue layout regressions cover
this behavior. See [final validation](../pr3-implementation-validation.md).
