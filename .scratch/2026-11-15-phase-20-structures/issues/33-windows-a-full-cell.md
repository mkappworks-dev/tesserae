# 33: Windows are at least a full cell of glass

Status: ready-for-agent

Blocked by: 22

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found the Templates' glass windows not reaching their sill. A
storey's window is a half-slab sill (`trim`, row 1) under one cell of glass
(row 2): the slab fills only the lower half of its cell, so a 0.5 m gap stands
open between the sill and the glass.

- Every window is at least one whole cell of glass with no gap round it: the
  sill is a whole cell, or the glass comes down to meet it.
- Larger buildings vary their windows: some 1 wide by 2 tall, some 2 by 2, the
  small houses keep 1 by 1. Each kind keeps its own look and Materials.
- Shapes come from the mockup's builders (`mockup/structures.js`), exported by
  `node mockup/export-templates.mjs`.

## Acceptance criteria

- [ ] No Template has a gap of air or a slab's open half between a window's
      glass and its sill, tested over every Game's Templates.
- [ ] At least the inn, warehouse, barn and farmhouse have windows 2 tall, and
      some 2 by 2, in the mockup and every Game.
- [ ] The walk, door, undo-budget and mockup export checks still pass.

## Comments
