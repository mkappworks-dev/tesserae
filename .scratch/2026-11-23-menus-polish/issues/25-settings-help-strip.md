# 25: Settings help strip

Status: ready-for-agent

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

- [ ] The strip shows the focused row's name, explanation and value note, and
      updates as focus or hover moves
- [ ] With nothing focused it shows the category description
- [ ] Rows use the full width and scroll under a fixed strip
- [ ] The strip matches the mockup on screen (capture)

## Comments
