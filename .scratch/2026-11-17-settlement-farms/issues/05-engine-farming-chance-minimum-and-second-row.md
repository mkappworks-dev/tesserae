# 05: Engine: farming chance, all-or-nothing minimum and a second row

Status: done

Spec: [Settlement farms](../spec.md).

## What to build

`Outskirts` gains `chance` (default 1). A Settlement draws from its hash whether
it farms; one that does lays its outskirts only when at least `count.0` of them
stand. Each slot is tried at the path, then a second time set back behind the
first row.

## Acceptance criteria

- [x] Over many Settlements with `chance` 0.5, between a third and two thirds
      farm, and each that farms stands at least the minimum.
- [x] With a minimum the ground cannot hold, no outskirt stands.
- [x] Fields stand two deep from an approach path somewhere in the test region.
- [x] Version 1 and 2 digests pass unchanged; outskirts stay the same in metres
      at 1 and 0.5 m.
