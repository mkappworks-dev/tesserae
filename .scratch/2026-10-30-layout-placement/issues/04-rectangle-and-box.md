# 04: Rectangle and box layouts

**What to build:** the second and third axes. The active axis can be cycled,
each axis has its own count and signed direction, and the preview shows the
rectangle or box with the 256 cap enforced visibly.

**Blocked by:** 03

**Status:** done

Spec: [layout placement](../spec.md).

- [x] Rectangle and box layouts preview and commit as one grouped Edit set.
- [x] A count that would exceed 256 Voxels is refused with feedback.
- [x] Axis cycling and direction flips work on keyboard and pad.
- [x] Reach clipping marks unreachable cells in every shape.

## Comments

### 2026-10-30: implemented

`H` cycles line, rectangle and box; `J` picks the active axis, which `[` / `]`
and `T` act on; `B` rotates which world axes the layout uses. Axes follow the
camera until one is changed: away from it, across it, then up. On a pad,
while the mode is on, D-pad left picks the axis, D-pad right reverses it and
D-pad down changes the shape; those buttons keep their Turn, Flip and empty
hand meaning with the mode off. A keyboard keeps Q, E and X throughout. An
increase past 256 Voxels is refused with a note in the readout. Not run on a
physical pad or reviewed visually.
