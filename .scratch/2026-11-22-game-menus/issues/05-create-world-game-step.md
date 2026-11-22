# 05: Create World: Game step and titles

Status: ready-for-agent

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

- [ ] The Game step lists the installed Games with Terra first and the current
      choice marked
- [ ] Leaving the Game step with an untrusted Game asks for trust; declining
      stays on the step
- [ ] The rail and the panel share one height
- [ ] Next and Create World sit in the same place on every step
- [ ] Enter activates the focused control; Enter on the already-chosen Game,
      or in a text field, moves to the next step
- [ ] Ctrl+Enter (⌘+Enter on macOS) or the controller's Start moves to the
      next step, or creates the World on Review, from anywhere on the step
- [ ] Review opens with Create World focused, so Enter creates
- [ ] The trust modal opens on Cancel; trusting takes a deliberate move to
      Trust
- [ ] Esc goes back a step, or cancels from the first step

## Comments
