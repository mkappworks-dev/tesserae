# 28: Jungle trees stand tall enough to walk under

Status: ready-for-agent

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found the jungle hard to cross: its trees are too short. The
`flora:jungle` tree (shared by Terra, Tidewater, Lumen and Meadow) has a 10 to
16 m trunk whose limbs start at 0.55 of it, lobes of 1.8 m, and solid vines
6 m long, so leaves, limbs and vines reach down to a Player's head.

- Make the jungle tree tall, in metres: a taller trunk, limbs starting
  higher, and vines that stop above a Player's head.
- Worldgen stays deterministic. There is no new Generator version: a World's
  snapshot pins its Plant rows, so a World saved before keeps its old trees and
  its recorded digests.
- If the change would move a recorded digest, stop and ask.

## Acceptance criteria

- [ ] Under every jungle tree worldgen stands, nothing of the tree but its
      trunk is lower than 4 m above its base, at 1.0, 0.5 and 0.25 m, tested.
- [ ] The tree stays within `MAX_PLANT_HEIGHT_M`.
- [ ] Every recorded digest test still passes unchanged.

## Comments
