# 05: Create World: Game step and titles

Status: done

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md), [03: Optional thumbnail in
mod.toml](./03-mod-thumbnail-field.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

Create World has five steps, starting with a Game step: a list of installed
Games with their thumbnails (or a neutral tile). The step rail is part of the
frame. Titles are short (Choose a Game, Name your World, Add Mods, Shape the
biomes, Ready to begin), only the Game step has a subtitle, and Next and Back
are prompts with one Next button. Leaving the Game step asks to trust the chosen
Game.

## Acceptance criteria

- [x] The Game step lists the installed Games with Terra first and the current
      choice marked
- [x] Leaving the Game step with an untrusted Game asks for trust; declining
      stays on the step
- [x] The rail and the panel share one height
- [x] Next and Create World sit in the same place on every step
- [x] Enter activates the focused control; Enter on the already-chosen Game, or
      in a text field, moves to the next step
- [x] Ctrl+Enter (⌘+Enter on macOS) or the controller's Start moves to the next
      step, or creates the World on Review, from anywhere on the step
- [x] Review opens with Create World focused, so Enter creates
- [x] The trust modal opens on Cancel; trusting takes a deliberate move to Trust
- [x] Esc goes back a step, or cancels from the first step

## Comments

- The Game step's Trust dialog is a new `Screen::StepTrust { on, to }` with
  `Act::TrustStep`; the Mods step still asks inline with its old panel and
  `Act::TrustMods`, left for ticket 08 to move onto the same dialog.
- The Ctrl+Enter cap reads "Ctrl+Enter" on every platform, as the mockup does;
  ⌘+Enter works on macOS without its own cap.
- Ctrl+Enter or Start drops whatever the focused control queued from the same
  Enter, so moving on never also picks the focused Game.
- Re-Create still opens on the World step, past the Game step, since its Game is
  fixed; ticket 10 moves it to Review.
- A component Game without a thumbnail shows the old landscape tile; ticket 06
  can swap in the terrain preview once the Game is trusted.
