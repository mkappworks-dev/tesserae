# 07: Columns

Status: ready-for-agent

Blocked by: 04

Spec: [Phase 18](../spec.md).

## What to build

The `column` kind: sugar cane, cactus, bamboo and kelp grow one Voxel at a time
up or down to `max-height-m`. A Sample that passes adds a Voxel at the end of
the run when the target is Air, or the fluid the Plant stands in, and the run is
under its height in metres. Removing a Voxel lets the run above regrow only
from the support rule, and a run whose base loses its support breaks from the
top, per ticket 06.

## Acceptance criteria

- [ ] A column reaches its height in metres, the same at 1.0, 0.5 and 0.25 m.
- [ ] Kelp stands in and grows through its fluid and not in Air; cane grows in
      Air and not into a solid.
- [ ] A column stops at its cap and at an obstruction without dying.
- [ ] Cutting the middle leaves the base to regrow and drops the cut Voxel's
      yield; the part above breaks by the support rule.
