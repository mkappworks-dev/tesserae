# 10: The hotbar is nine squares during play

Status: ready-for-agent
Blocked by: 09

Spec: [decision 20](../spec.md).

## What to build

During play the nine slots are squares at the bottom centre, drawn as the
menu's slot tiles are, with the selected one marked. An item held outside them
is a tenth square set apart beside them and marked instead of a slot. The held
item's name sits above the bar. The bar is hidden while the menu is open,
where the menu shows the same slots.

## Acceptance criteria

- [ ] The bar follows selection, the scroll wheel and an item held outside the
      slots, in headless Client tests.
- [ ] Its squares stay square at any window size.
- [ ] A capture in play is in the `/tmp` review folder.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
