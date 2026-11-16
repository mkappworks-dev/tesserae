# 32: A thumbnail of each Template in the place Tool's overlays

Status: done

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

- [x] The mockup's list rows and placement card show the thumbnail.
- [x] The catalogue carries each Template's thumbnail, drawn from its cells
      and the Materials' colours, tested: a Template's own Material colours
      appear in it and its background is clear.
- [x] The Client's list rows show it left of the text, and the placement card
      shows the chosen Template's, turned with the Structure, tested.

## Comments

Mockup: `structureThumb(id, turn)` (`structures.js`) draws each solid cell's
top and two near sides in its Material's texture; `.structure-thumb` holds it
on each `.structure-pick` row and in the placement card's head.

Client: `content::template_thumbnail` paints a 64 px RGBA isometric picture
from a Template's cells, the Materials' appearance colours (a `ground` cell
takes the World's surface colour), the top lit and the near sides shaded at
0.8 and 0.65, far cubes first. The Server draws all four turns into each
`CatalogueTemplate::thumbnails` when it publishes the Catalogue.
`structure_menu::make_thumbs` turns them into images when the Catalogue
changes; each list row shows the unturned one left of its text, and the
placement card's `Thumb` follows the chosen Template and the tool's turn.

Tests: `a_templates_thumbnail_shows_its_materials_on_a_clear_ground_and_turns_with_it`,
`the_placement_card_shows_the_chosen_templates_thumbnail_turned_with_it`, and
the published-catalogue test asks for a thumbnail at every turn. Checked in
play: Terra's fifteen Templates each show theirs in the list.
