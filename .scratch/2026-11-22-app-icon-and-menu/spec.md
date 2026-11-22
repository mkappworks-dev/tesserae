Status: ready-for-agent

# App icon and menu: Tesserae in the Dock and the menu bar

## Problem Statement

A Tesserae run shows a generic executable icon in the macOS Dock, and the menu
bar is winit's default: an app menu named after the binary with no About
Tesserae. The mosaic-T logo exists (`mockup/brand.svg`, drawn in the in-game
menu header) but the operating system never sees it. A release archive is a bare
binary, not an `.app` bundle, so no `Info.plist` or `.icns` can supply it.

## Solution

- A 1024 px icon, the mosaic T on a dark rounded tile, lives in `assets/` and is
  embedded in the binary.
- On macOS the app sets its Dock icon at startup.
- On macOS the menu bar gets an app menu (About Tesserae with the logo, Hide,
  Hide Others, Show All, Quit Tesserae) and a Window menu (Minimize, Zoom,
  Enter Full Screen). Quit takes the same path as the window's close button, so
  the save and unsafe-quit confirmation still apply.

## User Stories

1. As a Player on macOS, I see the Tesserae logo in the Dock and the Cmd-Tab
   switcher while it runs.
2. As a Player on macOS, the menu bar reads Tesserae, with an About panel that
   shows the logo and version.
3. As a Player, Cmd-Q behaves exactly like closing the window: a World saves
   first, and an unsafe quit still asks.

## Implementation Decisions

- The Dock icon is set at runtime through `NSApplication`, because there is no
  bundle. A later `.app` packaging would supersede this, not conflict with it.
- The menu is built with `muda`, after the event loop has launched: winit
  installs its default menu in `applicationDidFinishLaunching` and would replace
  an earlier one.
- Quit is a custom item that writes `WindowCloseRequested` for the primary
  window, not the predefined terminate item, which would skip the save flow.
- The in-game menu's change is decided separately; see ticket 02.

## Testing Decisions

- The icon bytes decode and are square, tested in the crate.
- The menu's Quit mapping (item id to `WindowCloseRequested`) is a pure function
  and tested; the AppKit calls are verified by running the app.

## Out of Scope

- An `.app` bundle, `.icns`, code signing and notarisation.
- A window icon on Windows and Linux, which needs a direct winit dependency
  and a platform to test it on.
- A Windows `.ico` embedded in the executable (the Explorer icon).

## Further Notes

Research files are not committed; this spec states what it needs.

## Comments
