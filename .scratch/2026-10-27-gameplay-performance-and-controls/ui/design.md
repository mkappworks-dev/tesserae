# Approved native menu reference

Status: ready-for-agent

The user approved leaf green, the five-tile Mosaic T logo, top-level Catalogue,
Behavior, Key bindings and Display tabs, and search on applicable pages. The
latest instruction requires layout, typography, colors and icons to match the
approved prototype. The HTML beside this record is the frozen reference; its
accent study and logo comparison panels are review tools outside the product.

The rendered dark reference uses Liberation Sans and Liberation Mono, background
`#0a0a0a`, foreground `#f5f5f5`, cards `#111111`, borders `#191919` and muted text
`#818181`. Leaf is the sRGB mix `#10b981` at 76% with `#fe9a00` at 24%, rendered
as approximately `#49b262`. Buttons use full selected borders; tabs use straight
2px underlines. The native UI bundles the reference fonts with their upstream
license and uses a shared vector icon atlas so missing glyphs cannot substitute
boxes.

Game-provided Materials, Shapes, textures and registry identity remain real
content. The reference's nine colored placeholder Materials do not replace the
Game's declarations. The saved-World Save & Quit action remains available in
the Catalogue footer, styled with the same UI primitives.

Native validation must compare the same content viewport sizes, retain offscreen
captures in a unique /tmp directory, cover focus and draft transitions and run
the Client regressions plus workspace CI checks. No WIT or Game source changes,
guest rebuilds, push, publication or merge belong to this UI pass.
