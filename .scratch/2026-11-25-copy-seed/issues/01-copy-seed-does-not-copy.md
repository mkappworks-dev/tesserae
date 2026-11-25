# 01: Copying a World's seed does not work

Status: needs-info

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

- [ ] Copying a seed on macOS puts it on the system clipboard, and pasting it
      elsewhere gives the same number.
- [ ] A failure still says "Could not copy the seed".

## Comments
