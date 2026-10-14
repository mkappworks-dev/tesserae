# The Server's edit path rejects unregistered Materials

Status: done
Blocked by: 01

The edit path's validation list says "the Material is not air (a Material
registry is step 7's)". The table is that registry for native code.

## What to build

- `RejectReason::UnknownMaterial`, raised by `MaterialTable::contains` the way
  `shapes.contains` raises an unknown Shape, in the same position in the
  validation order. The Client's text names it.

## Acceptance criteria

- Placing Material 999 is rejected and nothing changes; placing each built-in
  succeeds.
- The comment that defers the registry to step 7 is gone.
- `./scripts/cargo-ci` passes.

## Comments

Built as specified: `RejectReason::UnknownMaterial` ("unknown material" on
screen), checked first among the per-Voxel validations, before the Shape. A
test places Material 999 (rejected, nothing changes) and every built-in from 1
(each placed). The comment deferring the registry to step 7 was already gone.

