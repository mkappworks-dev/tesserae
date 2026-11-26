# 15: Declared water look

Status: ready-for-agent
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

- [ ] Data tests: a data water look decodes to what a component returns; bad
      values and non-fluid rows are refused naming the Mod and field.
- [ ] A loading test: Terra's declared values reach the per-slot table.
- [ ] Terra's lake, coast and underwater captures with its declared look, shown
      to the user.
