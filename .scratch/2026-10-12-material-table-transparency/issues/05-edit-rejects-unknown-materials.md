# The Server's edit path rejects unregistered Materials

Status: ready-for-agent
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
