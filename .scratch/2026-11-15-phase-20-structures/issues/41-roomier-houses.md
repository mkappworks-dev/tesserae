# 41: Taller storeys, roomier houses, lamps hung higher

Status: done

Spec: [Phase 20](../spec.md).

## What to build

Play-testing: "increase the ceiling height of each floor in a 2 story house.
have the lamp hanging bit high in both 1 story and 2 story building. make the
small house a little wider and breath and length. escpically 2 story house
because it is difficult to climb stairs."

- A building takes its storey height in cells; the family house and the inn use
  5 (4 m clear) instead of 4, and their stairs climb the taller storey.
- The cottage grows from 8 × 7 to 10 × 9 cells and the family house from 9 × 8
  to 11 × 10, their furniture moved to suit.
- Lamps hang a cell higher: under the upper floor on a ground floor, and a cell
  up into the roof under the top storey's roof.
- The mockup changes first; every Game's Templates are exported from it.

## Acceptance criteria

- [x] The family house and inn have floors at 0 and 5 cells, stairs reaching 5.
- [x] The cottage and family house are larger.
- [x] Every house lamp sits a cell higher than before.
- [x] The mockup's inside view bands follow each Template's storey height.
- [x] `node mockup/export-templates.mjs --check` passes.

## Comments

`building()` takes `storey` (default `STOREY`) and returns it with `top`, so
stairs and furniture are placed by storey rather than by fixed rows.
`storeysOf()` reads the storey height from the lowest upper floor.

Tidewater's fisher's cottage and harbour house keep the old 8 × 7 and 9 × 8
footprints (with the taller storey and higher lamps): at the larger size its
island Settlements laid too few Plots and the demo World test found one
building. `SET` takes an optional build per name for this. Terra's town lays
19 buildings within 96 m of its well at the test seed instead of 20+, so that
test now asks for 18.
