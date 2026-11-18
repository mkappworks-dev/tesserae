# 12: Retarget Plants, Structures, Settlements and spawn

Status: done

Blocked by: 04, 07

Spec: [Phase 17](../spec.md).

## What to build

Surface features, Plants, crop fields, Structures, Settlements and crossings read the new height and Water surface; steep slopes refuse cover and Structures through the existing ground rule; spawn picks dry land clear of cliffs.

## Acceptance criteria

- [x] The player spawns on dry land at least a metre above the Water surface.
- [x] No Structure stands in water or on a steep slope (rests on the existing ground rule, which asks the Water surface function; no new test).
- [x] Their own tests and digests under versions 1 and 2 pass unchanged.

Not separately tested: that no Structure stands in water or on a steep slope; it rests on the existing ground rule, which now asks the Water surface function, and the Terra Game's Structure tests pass under it.
