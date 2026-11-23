# 13: Map

Status: done

Blocked by: [02: Menu frame and prompt bar, on the title
screen](./02-menu-frame-and-prompt-bar.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

The Map fills the screen with its title at the top left and a compass. A legend
panel lists Waypoints (icon, name, distance) or the Explored biomes, switched
with Q and E using the same tabs as Key bindings. Add Waypoint, Centre, Zoom and
Close are prompts; a selected Waypoint adds Edit and Remove, except a death
marker cannot be removed.

## Acceptance criteria

- [x] Q and E switch the legend
- [x] Selecting a Waypoint in the legend adds its prompts
- [x] A death marker offers no Remove
- [x] Zoom works on the wheel and on + and −

## Comments

The toolbar and its tooltips are gone; Close is Esc (not the Map key, whose prompt would fire before the toggle and reopen the window). Zoom is a named-only `+ −` cap answered by Equal, Minus and the keypad. Moving onto a legend row (arrows or hover) selects it without opening it; the list rebuild keeps focus on the same Waypoint. The legend's tabs are pointer-only, as Q and E reach them. `frame::Cap::Text` is added with the same hunk as ticket 12's so the two merge cleanly. The `tooltip` capture state is removed.
