# 08: An item held outside the slots, F for Shape, and the controls listed

Status: ready-for-agent
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

- [ ] With an item held outside the slots, a click places it and the slots are
      unchanged.
- [ ] 1 to 9 and the scroll wheel clear it and select a slot.
- [ ] F cycles a held solid's Shape and wraps; Q and E still turn and flip it.
- [ ] F does nothing to a fluid or fire.
- [ ] The HUD text is one control per line and covers click, right click, 1 to
      9 and scroll, Q, E, F, Tab, movement, sprint, speed, return to spawn and
      Esc.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
