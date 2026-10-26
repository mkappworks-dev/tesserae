# 04: The demo declares its own ground, glass and leaves

Status: done
Blocked by: 03

Spec: [decision 7](../spec.md).

## What to build

The demo stops relying on built-in solids. It declares `demo:stone`,
`demo:dirt` and `demo:grass` as opaque solids, `demo:glass` (dimming 0, hiding
the same Material, drawn blended) and `demo:leaves` (dimming 1, hiding nothing,
drawn as cutout), each with today's placeholder colour. Its lamp takes the
lantern's near-white colour. It names stone, dirt and grass as its ground. Its
lava cools to `demo:stone`, and its stage and scenarios are built from its own
stone. `cargo dev` looks as it does today.

## Acceptance criteria

- [ ] The demo names no `core:` Material but air, by name or by id.
- [ ] Its generated ground is its own stone, dirt and grass.
- [ ] Lava touching its water cools to `demo:stone`.
- [ ] Every scenario plays out as before at 1.0, 0.5 and 0.25 m in the demo
      runtime tests.
- [ ] A `cargo dev` capture shows the same terrain, glass, leaves and lamp
      light as on main. Screenshots go in one `/tmp` folder for the user and
      are not committed.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

The demo declares its five solids with the former placeholder colours and
names its rock, soil and surface. Its lamp uses the former lantern colour;
its reactions, stage and scenarios use demo:stone. Existing content keeps
its registration order, with the five new solids appended. Runtime tests
expect the new ground and palette, and loaded fluid tests resolve the
reaction product by its demo name.

Guest rebuilding is underway. Scenario execution, formatting, full CI and
the visual comparison are deferred to the final verification pass per the
user's instruction. No screenshot has been reviewed yet.
