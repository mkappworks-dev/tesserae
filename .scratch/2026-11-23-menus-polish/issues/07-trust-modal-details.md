# 07: Trust modal: short form with Details

Status: done

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

The trust modal shows each item's name, build and Runs code, as the mockup does. A Details toggle expands the permissions and limits in a list that scrolls inside the modal; the modal never grows past the window. Cancel stays focused. The full Trust page (`Screen::Trust`) gets the same Details toggle, since it draws its rows from the same code.

## Acceptance criteria

- [x] Closed, the modal fits at 1280×720 with Terra and its required Mods
- [x] Details shows and hides the permissions and limits
- [x] Cancel is focused on open and Enter does not trust
- [x] The Trust page has the same Details toggle

## Comments

The Details terms are read from each item's `tesserae_mods::trust_prompt` (`pages::trust_terms`), so they show the item's effective limits rather than the mockup's fixed `TRUST_TERMS`. The modal's rows are compact; the Trust page keeps its larger cards. Captured at 1280×720 with Terra and its five required Mods untrusted (modal closed and open); the Trust page was checked by test only. Dev capture scripts gain `details` and `trust-step`.

