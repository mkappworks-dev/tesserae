# 10: Review and Edit World

Status: done

Blocked by: [07: Saved World thumbnail](./07-saved-world-thumbnail.md), [08:
Mods step](./08-mods-step.md), [09: Biomes step](./09-biomes-step.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Review shows Game, World, Mods and Biomes sections, each with an edit control
that jumps to its step, beside the terrain preview. Edit World uses the same
layout: the name and one Game mode to edit on the left; the World's thumbnail
and a summary of what was fixed at creation, including the seed number with a
copy control, on the right; New World with these settings at the bottom left and
Save changes at the bottom right, aligned with the form column; and prompts for
Save, New World with these settings, Show folder, Delete and Cancel.

## Acceptance criteria

- [x] Review, Edit World and World details show the seed number the World
      uses, with a control that copies it
- [x] Each Review section's edit control opens its step with the draft intact
- [x] Edit World fits without scrolling: name and the Game mode on the left,
      the preview and the fixed settings on the right
- [x] Save changes ends where the form column ends
- [x] Ctrl+Enter (⌘+Enter) or the Save button saves; Enter on a control
      still activates that control
- [x] While the Player is the World's only Player, Edit World shows one Game
      mode that sets both the World's default and the Player's own mode; the
      separate default appears only once other Players can join
- [x] Edit World saves the name and mode and shows the fixed settings read-only
- [x] New World with these settings is an outlined button at the bottom left,
      opposite Save changes, as well as the D prompt
- [x] New World with these settings opens Create World on Review with them
      filled in and a fresh name, Create World focused; nothing built in the
      World carries over and the same Game and Mods need no new trust
- [x] With an unsaved name or mode change it asks "Save changes first?"
      (Don't save, Save and continue) before leaving

## Comments

- World details is ticket 15's screen: it shows the seed with `pages::value_rows` and
  `Value::Seed(seed, Act::CopySeed)`, as Edit World does.
- Nothing lets another Player join a World yet, so Edit World always shows the one Game
  mode (`Act::EditMode` sets the default and the Player's own). The separate default
  returns with hosting.
- New World with these settings still passes each step's Trust: the same Game and Mods,
  already trusted, ask nothing; a World whose items were since untrusted asks once.
- The new World starts in the mode the World is set to, pending changes included.
- A rail jump past the Biomes step now reads the Biomes, since Review lists them.
