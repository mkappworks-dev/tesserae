# 10: Hunger and eating

Status: done

Blocked by: 09

Spec: [Phase 13.1](../spec.md).

## What to build

Survival Players get hunger. It drains per minute, faster while sprinting; health regenerates while hunger is at or above the threshold; at zero the Player starves. Using (G) a food Item in the selected slot eats one and refills hunger by its food value, refused with a notice when hunger is full. The HUD draws hunger as tiles right of centre above the hotbar, filling from the right.

## Acceptance criteria

- [x] Idle and sprinting drain at the declared rates; regeneration and starvation happen at their thresholds.
- [x] Eating spends one food Item and refills hunger, capped at the maximum; eating when full is refused.
- [x] Creative Players' hunger does not drain.
