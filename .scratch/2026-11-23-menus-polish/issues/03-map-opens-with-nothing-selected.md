# 03: Map opens with nothing selected

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

Opening the Map clears the Waypoint selection, so the Map opens centred on the Player with the browse prompts first, as in the mockup.

## Acceptance criteria

- [x] Opening the Map leaves no Waypoint selected and no Edit or Remove prompt
- [x] The view still follows the Player on open

## Comments

Opening also drops a pan from the last visit (`centre = None`), so the view follows the Player even if the window was closed while panned.
