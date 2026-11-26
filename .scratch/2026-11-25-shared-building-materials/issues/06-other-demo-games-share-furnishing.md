# 06: The other demo Games use the shared Stations and furnishing

Status: done

Blocked by: 04

Spec: [Shared building Materials](../spec.md).

## What to build

Ember, Lumen, Meadow and Tidewater depend on `workshop`, `timber` and
`glassworks`, and use `workshop`'s Stations and chest, `timber`'s door and
`glassworks`' glass and lantern in place of their own. Every themed building
Material stays the Game's own, with its Recipe now opened by the shared
Stations; a Recipe that named the Game's own glass names `glassworks:glass`. The
shared demo building kit stops declaring Stations, chests, Doors, glass, lamps
and its `panel` and `table` Shapes. The mockup's furnishing table names the
shared furnishing for every Game, and their Templates are regenerated. Loot that
named a Game's own furnishing names the shared Item.

No new tests: demo-Game tests run on Terra only. CI's validate step over
`games/*/` (ticket 01) is the check.

## Acceptance criteria

- [x] All five demo Games pass CI's validate step.
- [x] No demo Game declares a Station, chest, Door, glass or lamp of its own.
- [x] The generator's check mode reports no drift for any Game.
