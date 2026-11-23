# 20: macOS window without a title bar

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

On macOS the game draws under a transparent title bar: `fullsize_content_view`,
`titlebar_transparent` and no title text, set when the window is created
(macOS reads them only then). The window buttons stay over the scenery at the
top left, and dragging the top strip still moves the window. Menus keep the
buttons' strip clear so no title or control sits under them. Windows and Linux
keep their native frame; ticket 21 adds the window mode setting for them.

## Acceptance criteria

- [ ] On macOS the scenery reaches the top edge with no title bar or title text
- [ ] Close, minimise and full screen work from the window buttons; the window
      still drags by its top strip
- [ ] No menu title, prompt or control sits under the window buttons, checked
      by on-screen capture of the title screen, a Create World step, Settings and
      the pause menu
- [ ] Other platforms are unchanged

## Comments
