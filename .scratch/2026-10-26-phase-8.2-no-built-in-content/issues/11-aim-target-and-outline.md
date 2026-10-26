# 11: The Server answers where the player aims, and the Client outlines it

Status: ready-for-agent
Blocked by: 07

Spec: [decisions 15 and 16 (the outline)](../spec.md).

## What to build

The Client sends an aim report when its ray moves: a sequence number, an
origin and a direction in metres, and the held item. The Server drops it when
not finite, stale, or too far from the Eye, as it does an edit intent. Once a
tick, for the latest report, it answers with an aim target worked out by the
same ray and layer rules as ticket 07's remove and the place path:

- the outline: the metre bounds of the Shape a remove would take, or nothing;
- the ghost cell: where a click would put the held solid or source, only when
  that cell would accept it;
- for fire, the outline of the solid a click would ignite, and no ghost cell.

The Client draws the outline as a thin box of lines and hides it while the
menu is open. The answer holds no Voxel or Chunk (ADR-0002).

## Acceptance criteria

- [ ] Server tests cover outline bounds for a cube, slab and stair; the ghost
      cell present and absent; fire with no ghost; stale and far reports
      dropped.
- [ ] For a set of rays, the aim target agrees with what the matching edit
      intent then does.
- [ ] The Client draws one outline at the target and none with no target or
      with the menu open.
- [ ] A capture showing the outline is in the `/tmp` review folder.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
