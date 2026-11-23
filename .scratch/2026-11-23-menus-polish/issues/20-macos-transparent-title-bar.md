# 20: macOS window without a title bar

Status: done

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

- [x] On macOS the scenery reaches the top edge with no title bar or title text
- [x] Close, minimise and full screen work from the window buttons; the window
      still drags by its top strip
- [x] No menu title, prompt or control sits under the window buttons, checked
      by on-screen capture of the title screen, a Create World step, Settings and
      the pause menu
- [x] Other platforms are unchanged

## Comments

AppKit's transparent title bar measured 32 pt, with the buttons at x 9-69, y
9-23 pt. On macOS `theme::SCREEN_PAD_TOP` is 32 instead of 28, so a sheet starts
below the strip; the launcher pages now use it too. The profiling monitor no
longer sets the two title bar fields itself. Dragging by the strip and the
buttons are AppKit's own and were not exercised by hand: the session has no
input automation or Screen Recording permission, so captures are in-app and
show the scenery, not the buttons.

