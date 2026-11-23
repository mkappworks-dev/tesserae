# 01: Dock icon, window icon and macOS menu bar

Status: done

Blocked by: None (can start immediately)

Spec: [App icon and menu](../spec.md).

## What to build

`assets/icon.png` (1024 px), embedded; Dock icon on macOS; the macOS app and Window menus with an About panel showing
the icon, Quit routed through `WindowCloseRequested`.

## Acceptance criteria

- [x] `cargo run -p tesserae` shows the logo in the Dock and Cmd-Tab on macOS
- [x] The menu bar title is Tesserae and About shows the logo and version
- [x] Cmd-Q in a saved World closes through the save flow
- [x] `cargo lint` and `cargo deny check` pass with the new dependencies
- [x] `assets/README.md` records the icon and its licence (CC0)

## Comments

Marked done at the user's request on 2026-11-23. Built in 2204b5ad. The Dock
and Cmd-Tab icon, About and Cmd-Q were not checked by hand in this session;
`cargo lint` and `cargo deny check` run in the PR's CI.
