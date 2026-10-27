# 08: An item held outside the slots, F for Shape, and the controls listed

Status: done
Blocked by: 07

Spec: [decisions 17, 18 and 20 (the control list)](../spec.md).

## What to build

The hotbar gains an optional item held outside its nine slots. When set it is
what a click places; selecting a slot by 1 to 9, the scroll wheel or a click
on a slot clears it. Q, E and F act on whichever solid is held. F cycles it
through the catalogue's Shapes in id order, wrapping and keeping its
orientation, and does nothing to a fluid or fire. The bottom-right HUD lists
the controls one per line, with notices on their own line under them. The
camera's look-around prompt and the menu's header are split the same way.
Text stays ASCII.

## Acceptance criteria

- [x] With an item held outside the slots, a click places it and the slots are
      unchanged.
- [x] 1 to 9 and the scroll wheel clear it and select a slot.
- [x] F cycles a held solid's Shape and wraps; Q and E still turn and flip it.
- [x] F does nothing to a fluid or fire.
- [x] The HUD text is one control per line and covers click, right click, 1 to
      9 and scroll, Q, E, F, Tab, movement, sprint, speed, return to spawn and
      Esc.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

Hotbar carries an optional outside item; held reads it before the selected
slot. Slot selection and actual wheel steps clear it. Q/E/F operate on the
held solid, and F cycles catalogue Shape ids in order while preserving its
orientation. Controls and the camera prompt use one ASCII control per line;
notices remain a separate line. Menu quick pick follows in ticket 09.
Formatting, input-suite review and full CI remain deferred to ticket 13.

### 2026-10-26: final verification

Checked on the completed phase tree in ticket 13. Formatting and full CI
pass; the final captures were reviewed and accepted by the user. See
13-final-verification.md for benchmark results and validation exceptions.
