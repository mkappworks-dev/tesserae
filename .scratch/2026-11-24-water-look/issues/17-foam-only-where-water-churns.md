# 17: Foam only where water churns

Status: done
Blocked by: 16

Spec: [Water that looks natural](../spec.md), Foam. From the user's review of
the finished branch.

## What to build

Still water never foams. The shore band foams only on the sea, where its swash
churns the beach; a lake's margin, a shallow pond and a Settlement's garden
pond are as clear at the edge as in the middle. Breakers on the sea, a Fall's
landing and fast, steep water still foam.

## Acceptance criteria

- [ ] The shore band is scaled by the sea's share of the columns.
- [ ] A capture of a Settlement pond and a lake's margin with no foam, and the
      coast's shore with its band, shown to the user.

## Comments

- 2026-11-27: the shore band is scaled by the sea's share, and it, a breaker
  and a Fall's landing foam at most 0.6 of the surface, so the mask breaks
  them into lace. The crop field's canals in the user's World lost their
  white band. Captures in `/tmp/tesserae-water-look-captures/18-blackspot/field-yaw270.png` and
  `/tmp/tesserae-water-look-captures/20-day/`.
