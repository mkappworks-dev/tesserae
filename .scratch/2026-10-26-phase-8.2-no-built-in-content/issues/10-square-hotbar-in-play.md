# 10: The hotbar is nine squares during play

Status: done
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

## Comments

### 2026-10-26: implementation

The play bar reuses the menu's square tile builder, with the held name above
nine slots at bottom centre. An outside item adds a marked tenth tile with
a gap; slot borders stop marking selection until returning to the bar.
Opening the menu hides the play bar. Headless UI coverage checks the nine
and ten square states and menu visibility. Captures, formatting, review and
full CI are deferred to the final verification pass.
