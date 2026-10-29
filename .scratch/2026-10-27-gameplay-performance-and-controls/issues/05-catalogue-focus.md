# 05: C2: Catalogue selection and assignment

Status: done

Spec: [catalogue and readable UI](../spec.md#catalogue-and-readable-ui).
Implementation: merged PR #24, main `4b86e43`.

## Completed scope

Keep the existing catalogue selection and assignment workflow. Clicking a
Material holds it while leaving Catalogue open; clicking it again or using
Clear selection empties the hand. Choosing a Shape updates the held variation.
Assign to slot, mouse drag/drop and number shortcuts organize the quick slots.
An explicitly selected Material pins its Shape panel while the pointer passes
other entries. Drag sources remain alive until their drag completes.

## Acceptance criteria

- [x] Select a Material and Shape without closing Catalogue.
- [x] Assign content through the existing visible action, drag/drop and numbers.
- [x] Explicit Material selection keeps its Shape panel during other hovers.
- [x] Drag/rebuild handling preserves the source until drag completion.
- [x] Catalogue interactions do not click through to World edits.

## Completion decision

The user accepted the existing main behavior as complete and requested removal
of the expanded PR 3 UI requirements after reverting that implementation.
This narrowed scope is authoritative. No additional UI work remains in this
ticket; do not reintroduce the rejected implementation as a follow-up task.
Controller and desktop-touch work remains in its separately scoped tickets.

Existing Client selection, assignment, drag and edit-intent regressions cover
this behavior. See [final validation](../pr3-implementation-validation.md).
