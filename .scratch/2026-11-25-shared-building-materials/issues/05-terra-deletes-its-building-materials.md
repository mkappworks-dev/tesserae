# 05: Terra deletes its own building Materials

Status: ready-for-agent

Blocked by: 04

Spec: [Shared building Materials](../spec.md).

## What to build

Terra stops declaring the Materials ticket 04 left unused: every building
Material in the spec's mapping table, its Stations, chest, Doors, `glass` and
`lamp`, with their Items, Recipes and voxel-uses, and its `panel` and `table`
Shapes. `fence`, `terra:ash` and `terra:iron` stay. Tests that named a deleted
Material name its shared replacement or a Material Terra still declares,
including the loot, Container, mod-loading, runtime registration and reaction
tests. The mockup's exported Material gallery is regenerated from the new set.

## Acceptance criteria

- [ ] Terra declares none of the deleted Materials or Shapes, and nothing in the
      workspace names them.
- [ ] The full test suite passes.
- [ ] The mockup's Material gallery lists the shared Materials Terra uses and
      none of the deleted ones.
- [ ] Terra's boundary check passes.
