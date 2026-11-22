# 01: Dock icon, window icon and macOS menu bar

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [App icon and menu](../spec.md).

## What to build

`assets/icon.png` (1024 px), embedded; Dock icon on macOS; the macOS app and Window menus with an About panel showing
the icon, Quit routed through `WindowCloseRequested`.

## Acceptance criteria

- [ ] `cargo run -p tesserae` shows the logo in the Dock and Cmd-Tab on macOS
- [ ] The menu bar title is Tesserae and About shows the logo and version
- [ ] Cmd-Q in a saved World closes through the save flow
- [ ] `cargo lint` and `cargo deny check` pass with the new dependencies
- [ ] `assets/README.md` records the icon and its licence (CC0)

## Comments
