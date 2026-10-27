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

- [x] The bar follows selection, the scroll wheel and an item held outside the
      slots, in headless Client tests.
- [x] Its squares stay square at any window size.
- [x] A capture in play is in the `/tmp` review folder.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

The play bar reuses the menu's square tile builder, with the held name above
nine slots at bottom centre. An outside item adds a marked tenth tile with
a gap; slot borders stop marking selection until returning to the bar.
Opening the menu hides the play bar. Headless UI coverage checks the nine
and ten square states and menu visibility. Captures, formatting, review and
full CI are deferred to the final verification pass.

### 2026-10-26: user amendment during final capture review

The user requested model previews in the menu and hotbar quick slots,
superseding the original flat-colour tile decision. Ticket 13 implements a
shared content-only model atlas for the tiles. The corner model was also
reduced to scale 0.06 after two rounds of visual feedback.

### 2026-10-26: final verification

Checked on the completed phase tree in ticket 13. Formatting and full CI
pass; the final captures were reviewed and accepted by the user. See
13-final-verification.md for benchmark results and validation exceptions.

### 2026-10-26: hover and scrolling follow-up for PR #21

The user requested a visible hover state and quick slots fixed at the
bottom centre, with the item selection scrolling independently. Tiles use
a brighter hover background and an accent border, retaining the selected
slot's white border. The selection uses Bevy's ScrollArea and a vertical
scrollbar; slots live in a separate footer. Assigning slots preserves the
browse position. Open-menu wheel input no longer selects a hotbar slot or
clears an outside-held item.

Added pointer-enter/exit styling coverage, native line/pixel scroll input
with a stationary footer and retained browse position, and a Client input
test for wheel events while the menu is open. Core code, Game/WIT content
and simulation bounds are unchanged, so the previous core A/B still applies.
Formatting and full CI pass: 878 tests passed, five existing measurement
tests remain ignored. The user reviewed and accepted the hover, scrolled
and full-size menu captures on 2026-10-26; the /tmp review folder was then
deleted.
