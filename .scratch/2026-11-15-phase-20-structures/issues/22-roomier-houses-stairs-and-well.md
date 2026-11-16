# 22: Roomier houses, stairs and a working well

Status: ready-for-agent

Blocked by: 17, 21

Spec: [Phase 20](../spec.md).

## What to build

The user reviewed the Games' Settlement buildings in play and found three
problems. The shapes come from the mockup's builders (`mockup/structures.js`),
exported by `mockup/export-templates.mjs`, so every fix lands in the mockup
first and is then exported.

- **Interiors are too small.** Widen and deepen every house kind and raise its
  storey height, so a Player walks round its furniture and under its beams with
  headroom to spare. That means at least 2.5 m clear inside each storey and 2 m
  clear floor beyond each piece of furniture.
  - Keep each kind's own shape: steep-roofed cottage, jettied family house,
    L-shaped farmhouse with porch and cellar, hipped inn, long warehouse,
    lean-to blacksmith, bakery with its oven at the back, market stall and
    barn.
  - Keep every Template within the Settlement radius and within the undo
    budget at 0.25 m, which is 131,072 changed Voxels. If a kind cannot fit
    both, say which and by how much.
- **Upper storeys cannot be reached.** Every building with an upper storey or
  a cellar gets a stair, built from the Game's stair Shape in a wall Material,
  with a matching opening in the floor above. Each storey and cellar can be
  walked to from the door, with no jumping. A loft reached by ladder is fine
  only if the Game declares a climbable ladder.
- **The well.** Its two posts stand on the well's base, with no gap under them.
  It holds water from its floor up to about 1 m below its rim, so a Player sees
  water when looking in. Every Game's well does this: Terra's well,
  Tidewater's tide well, Lumen's moon well, Meadow's pond well and Ember's
  equivalent.
  - The water is a Template cell of the Game's water Material, placed as a
    source.
  - Find out whether a Template cell may be a fluid. If the loader or the
    Obtainable check refuses it, fix the rule, not the content, and record the
    fix.
- Settlement radii and spacings grow only as far as the larger buildings need.

## Acceptance criteria

- [ ] Every house kind has at least 2.5 m headroom per storey and the larger
      footprint, in the mockup and in every Game. The mockup export check
      passes.
- [ ] A test walks from each building's door to every storey and cellar, over
      Voxels a Player can stand on and step up, with no gap over 1 step.
- [ ] Each well's posts rest on its base, and its shaft holds water, tested
      through the Game's Templates.
- [ ] Every shipped Template still fits the undo budget at 0.25 m, tested.
- [ ] The Settlement tests, the benchmark margin and the Obtainable check
      still pass.
