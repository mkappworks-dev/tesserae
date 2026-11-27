# 15: Declared water look

Status: done
Blocked by: 09, 10

Spec: [Water that looks natural](../spec.md), Declared water look.

## What to build

A Game or Mod declares how its water looks. An optional water look per Material
joins contract line 0.3 beside `appearance`: absorption per channel in m⁻¹,
scatter tint, roughness and foam colour, each optional, defaults filling the
rest. Data-only Mods declare it in TOML. A row naming a Material that is not a
blended Fluid is refused. The Server's water-look message from 08 carries the
declared values to the per-slot table and the underwater fog. The WIT copies in
the starters and the contract, authoring and data-Mod docs follow, as they did
for the colour branch's appearance change. Terra declares a look tuned on
captures; `waters` declares pure water, which Ember, Lumen, Meadow and Tidewater
draw.

## Acceptance criteria

- [x] Data tests: a data water look decodes to what a component returns; bad
      values and non-fluid rows are refused naming the Mod and field.
- [x] A loading test: Terra's declared values reach the per-slot table.
- [x] Terra's lake, coast and underwater captures with its declared look, shown
      to the user.

## Comments

- 2026-11-27, on 85287a6b plus this ticket:
  - A list on the `declaration` record, `water-looks`, rather than a field of
    `appearance`: `appearance` is built in many places in every Game, a
    Declaration in one. Each `water-look` names a local Material and has
    optional `absorption-per-m` (three `f32`, each 0 to 100), `scatter` and
    `foam` (`srgba`, alpha ignored) and `roughness` (0.02 to 1); at most 16 a
    Mod. Data-only Mods write `[[water-looks]]`. The WIT copy in the starter
    follows, and the contract, authoring and data-Mod docs.
  - Bounds are checked as the Mod is applied; whether a row names one of the
    Mod's own blended Fluids once every Mod's Fluids are known, refused as
    `mod `x`: water-looks[0].material `rock` is not a blended Fluid`. Declared
    values lay over pure water's and over the default a Material tagged `water`
    gets; they travel in 08's `MaterialWaterLooks`, so the per-slot table and
    the underwater fog take them alike.
  - Terra declares absorption 0.42, 0.07 and 0.02 m⁻¹, scatter sRGB (20, 70,
    105) and foam (236, 240, 240): a deep lake and the open sea turn
    blue-teal, and the shallows still show the sand. A first try at 0.45,
    0.09 and 0.05 hid the lake's floor from the underwater Eye. `waters`
    declares pure water, an empty row, which Ember, Lumen, Meadow and Tidewater
    draw.
- Captures in `/tmp/tesserae-water-look-captures/15-after/` (`sheet.png`: lake,
  coast, underwater). Under water the view is a blue-teal fog with the floor
  faint past 30 m; Terra's scatter tint is where to take it bluer or clearer.
  They wait on the user's review.
- 2026-11-27: the user reviewed the captures and approved them.
