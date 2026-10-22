# 06: Apply declared fluid contact reactions atomically

Status: ready-for-agent
Blocked by: 04, 05

Spec: [decisions 6, 9 and 11](../spec.md).

## What to build

Water contacting lava produces the demo's declared solid result, and a
second declared pair behaves through the same native mechanism. Reaction
results respect geometry and never partially consume their inputs.

## Acceptance criteria

- [ ] Resolve reaction participant/result names before World open and reject
  mirrored duplicates or conflicting overlapping definitions.
- [ ] Contact requires accessible geometry. Adjacent fluid regions separated
  by a solid partition neither react nor mix through it.
- [ ] Source/flowing matching and receiving-side rules are explicit data.
  Simultaneous contacts use stable coordinate/face order with revalidation.
- [ ] The demo's water/lava reaction replaces contacted lava with existing
  stone and preserves water. A custom pair proves it is not hardcoded.
- [ ] Solid products use placement validation and shaped caps and require
  available solid space. A reaction cannot destroy a waterlogged stair/fence
  to make room for its product.
- [ ] Whole results land atomically. A stale dependency, blocked product, or
  unavailable result consumes neither participant.
- [ ] Unchanged blocked contacts sleep until inputs change; retries, wakeups,
  and retained results remain budgeted.
- [ ] Native, real-Mod, cross-Chunk, stale-Job, and shared-path tests verify
  results, light, mesh delivery, and level-1 invalidation.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Reaction policies remain Game/Mod data. This ticket does not add recipes,
  inventories, or automatic temperature-driven phase transitions.
