# Approved native menu reference

Status: done

The user approved leaf green, the five-tile Mosaic T logo, top-level Catalogue,
Behavior, Key bindings and Display tabs, and search on applicable pages. The
latest instruction requires layout, typography, colors and icons to match the
approved prototype. The HTML beside this record is the frozen reference; its
accent study and logo comparison panels are review tools outside the product.

The user subsequently selected public-domain Aileron for every UI label. The
reference embeds the same unchanged Regular, SemiBold and Bold files as the
native menu. The default native Font asset is Aileron Regular for HUD text and
save dialogs. This supersedes Liberation Sans/Mono in earlier review captures.
There is no separate monospace family; keys and values use Aileron too.

Background is `#0a0a0a`, foreground `#f5f5f5`, cards `#111111`, borders `#191919`
and muted text `#818181`. Leaf is the sRGB mix `#10b981` at 76% with `#fe9a00`
at 24%, approximately `#49b262`. Buttons use full selected borders; tabs use
straight 2px underlines. A shared vector icon atlas prevents missing glyphs.

Clicking a catalogue item pins its details and keeps the menu open without
changing the hand. The explicit Hold action returns to play; assignment stays
in the menu. Tiles have fixed 128 × 113 logical-pixel dimensions and wrap to fit
the available width, superseding the reference's original three-column grid.

Game-provided Materials, Shapes, textures and registry identity remain real
content. The reference's nine colored placeholder Materials do not replace the
Game's declarations. The saved-World Save & Quit action remains available in
the Catalogue footer, styled with the same UI primitives.

Native validation must compare the same content viewport sizes, retain offscreen
captures in a unique /tmp directory, cover focus and draft transitions and run
the Client regressions plus workspace CI checks. No WIT or Game source changes,
guest rebuilds, push, publication or merge belong to this UI pass.


## Implementation evidence

Implemented through `b1e7d02`. All 143 Client regressions passed; the full CI
recheck passed 967 tests with zero failures and five existing measurements
ignored. See `../pr2-implementation-validation.md` for the earlier intermittent
persistence failure and complete check details.

Twelve retained native offscreen captures, the executable, source stamp,
font hashes and environment manifests are under
`/tmp/tesserae-leaf-review-7udu289a/final/`. The review entry point is
`/tmp/tesserae-leaf-review-7udu289a/native-review.html`. Fixed tile dimensions,
five/six-column wrapping, all four pages, search and modal states were reviewed.
The dark palette, leaf accent, Mosaic T, layout dimensions, Aileron family and
icon shapes follow the updated reference. Native text rasterization and real
Game previews differ from browser placeholder rendering.
