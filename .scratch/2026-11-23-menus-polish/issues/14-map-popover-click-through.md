# 14: Map: a click elsewhere moves the point or opens a marker

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

The Map is exempt from ticket 13's rule that the closing click does nothing else, as in the mockup: with the add popover open, a click on explored ground moves its point; a click on another marker opens that marker's popover; a click on the legend or prompt bar acts as usual. Every other menu popup keeps ticket 13's rule.

## Acceptance criteria

- [x] With the add popover open, a click on the Map moves its point
- [x] With a popover open, a click on another marker opens that marker's popover
- [x] Mods and Biomes popups still close without acting on the closing click

## Comments

The Map popover is no longer a `frame::Popup`, so no scrim covers the Map and its own click handling runs as before ticket 13. A click elsewhere on the Map window's chrome leaves the popover open. The mockup's outside-click rule still listed `#map-pop`, contradicting this ticket; it now exempts it too.
