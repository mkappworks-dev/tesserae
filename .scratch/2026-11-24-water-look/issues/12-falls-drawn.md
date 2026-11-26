# 12: Falls drawn

Status: ready-for-agent
Blocked by: 11

Spec: [Water that looks natural](../spec.md), Falls drawn.

## What to build

Water pours over every generated lip that has a lower pool past it. The mesher
emits a thin blended sheet of the upper pool's water on the lip's far face, a
sixteenth off the rock, from the upper Water surface to the lower, and the lower
pool's columns beside it are flagged for foam. Vertical faces of a water
Material draw falling Flow, so a Settlement's simulated waterfall (Terra's
`waterfall` Outskirts Template) looks the same. Nothing is added to the World.

## Acceptance criteria

- [ ] Meshing tests: a lip with a lower pool past it emits one sheet spanning
      the two surfaces; a lip beside dry ground emits none; the pool below is
      flagged for foam.
- [ ] Captures at the river fixture and a Settlement waterfall, shown to the
      user.
- [ ] At the river fixture, how many frames a blended mesh waits for its sort,
      against 11, in Comments.
