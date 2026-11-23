# 07: Trust modal: short form with Details

Status: ready-for-agent

Blocked by: [02: Approve the mockup](./02-mockup-approval.md)

Spec: [Menus polish](../spec.md).

## What to build

The trust modal shows each item's name, build and Runs code, as the mockup does. A Details toggle expands the permissions and limits in a list that scrolls inside the modal; the modal never grows past the window. Cancel stays focused. The full Trust page (`Screen::Trust`) gets the same Details toggle, since it draws its rows from the same code.

## Acceptance criteria

- [ ] Closed, the modal fits at 1280×720 with Terra and its required Mods
- [ ] Details shows and hides the permissions and limits
- [ ] Cancel is focused on open and Enter does not trust
- [ ] The Trust page has the same Details toggle

## Comments
