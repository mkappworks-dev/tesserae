# 01: Copying a World's seed does not work

Status: done

## What happens

Reported by the user on 2026-11-25: in a World's details, the copy action for
the seed does not copy it, so the seed cannot be shared or pasted elsewhere.

The copy lives in `client/worlds/edit_world.rs` (`copy`) and, in game, in
`client/menu.rs` (`copy_seed`); both call Bevy's `Clipboard::set_text` and
show "Seed copied" or "Could not copy the seed". It shipped with the game
menus (#73), so `main` has it too.

## Needs from the reporter

- Which notice shows after copying: "Seed copied" (the clipboard took it, so
  the paste is what fails) or "Could not copy the seed" (no clipboard, or it
  refused).
- Which screen: the World list's details, or the in-game menu.

## Acceptance

- [x] Copying a seed on macOS puts it on the system clipboard, and pasting it
      elsewhere gives the same number.
- [x] A failure still says "Could not copy the seed".

## Comments

2026-11-25: the user reports "Seed copied" shows on every screen with a copy,
and the paste elsewhere is empty. Cause: `bevy_clipboard` 0.20 writes to an
in-process buffer unless Bevy's `system_clipboard` feature is on (it pulls in
`arboard`); `crates/tesserae/Cargo.toml` asks Bevy for `serialize` only, and
`cargo tree -i arboard` finds nothing. The fix is that feature, on its own
`fix/` branch from `main`, since #73 shipped it there; check `cargo deny` for
`arboard`'s dependencies.

2026-11-26: fixed on `fix/colour-art-and-shader` rather than its own branch:
the user plays this branch, and `main` takes it when this merges. Bevy's
`system_clipboard` feature is on in `crates/tesserae/Cargo.toml`; `arboard`
was already in the lockfile, and `cargo deny check` passes. Checked on macOS
with a throwaway test (not committed): Bevy's `Clipboard::set_text` with a
seed, then `pbpaste` gave the same number. The failure notice is unchanged.
