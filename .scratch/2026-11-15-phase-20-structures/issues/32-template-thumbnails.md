# 32: A thumbnail of each Template in the place Tool's overlays

Status: ready-for-agent

Blocked by: 11, 29

Spec: [Phase 20](../spec.md).

## What to build

Play-testing asked for a preview beside each Template in the place Tool's
list, and on the placement card shown in play, so a Player sees what they pick
without placing it.

- **The list:** each Template's row shows a small isometric thumbnail of it to
  the left of its name, size and kind.
- **The placement card** (the HUD card in play with the place Tool in hand):
  the chosen Template's thumbnail stands beside its name and facing, turned as
  the Structure will be placed.
- The thumbnail is drawn from content alone, the Template's cells and each
  Material's colour, so it needs no World and no render pass: Keep and Air
  cells are left out, a `ground` cell takes a neutral earth colour.
- The mockup changes first, then the Client.

## Acceptance criteria

- [ ] The mockup's list rows and placement card show the thumbnail.
- [ ] The catalogue carries each Template's thumbnail, drawn from its cells
      and the Materials' colours, tested: a Template's own Material colours
      appear in it and its background is clear.
- [ ] The Client's list rows show it left of the text, and the placement card
      shows the chosen Template's, turned with the Structure, tested.

## Comments
