# 30: The Use prompt and the place hint no longer overlap

Status: done

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found, aiming at a Door with a block in hand, "G Open Door" and
"Right mouse to place" drawn over each other under the reticle. The place hint
stands 24 px below the reticle and the Use prompt 37 px, so they overlap; the
mockup has the same overlap (`.use-hint` at 26 px, the place hint at 24 px).

- Under the reticle, the Use prompt and the place hint stack in one column
  with a gap, the Use prompt first, each centred. Either shows alone where it
  stood before.
- The mockup changes first, then the Client.

## Acceptance criteria

- [x] The mockup shows both lines stacked when both apply.
- [x] The Client lays both out in one column, tested: their boxes do not
      overlap when both are shown.

## Comments

Mockup: `.reticle-hints` (`mockup/crafting.css`) is the column under the
reticle, 24 px down with a 6 px gap; `play()` puts `useHint()` and the place
hint in it. Client: `use_prompt::spawn` makes the same column and holds the
prompt's row and `removal::place_hint_bundle()`; the prompt's row collapses
with `Display::None` instead of hiding, so the place hint moves up into its
place. Checked in play, aimed at a chest with leaves in hand: "G Open Chest"
over "Right mouse to place", apart.
