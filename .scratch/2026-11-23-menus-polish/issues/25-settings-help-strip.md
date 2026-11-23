# 25: Settings help strip

Status: done

Blocked by: [21: Window mode in Display settings](./21-window-mode-setting.md)
(both change Settings)

Spec: [Menus polish](../spec.md).

## What to build

Settings' help panel moves from the right-hand callout to a strip along the
bottom of the sheet, as in the approved mockup (`/settings/behavior`): a band in
the rail's tone with a thin top rule, fixed above the prompt bar while the rows
scroll, and the rows take the full width. It shows the focused or hovered
setting's name (bold, one step larger), its explanation (muted) and, where the
current value has one, a value note in the light accent colour with the value
bold. With nothing focused it shows the category's one-line description. The
per-value notes and the new row explanations come from the mockup's
`settings.js`.

## Acceptance criteria

- [x] The strip shows the focused row's name, explanation and value note, and
      updates as focus or hover moves
- [x] With nothing focused it shows the category description
- [x] Rows use the full width and scroll under a fixed strip
- [x] The strip matches the mockup on screen (capture)

## Comments

The strip explains a row only under the pointer or with a visible focus
(keys, controller). The automatic focus on opening a category is invisible to
a mouse Player, so the strip opens on the category description; the mockup's
browser instead rings and explains the first row on load.
