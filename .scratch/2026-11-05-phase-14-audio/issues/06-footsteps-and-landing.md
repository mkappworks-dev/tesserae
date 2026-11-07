# 06: Footsteps and landing

Status: done

Blocked by: 02

## What to build

A Player hears their footsteps on the Material under their feet and a landing
after a real fall. The Server measures both from the body it already advances.

- `step` every 1.5 m of horizontal travel while grounded and not crouching,
  from the solid under the feet. Flight and swimming make none. Sprinting keeps
  the stride.
- `land` when the body becomes grounded after falling 1 m or more, from the
  solid landed on.
- Stride and fall height are engine constants in metres.

## Acceptance criteria

- [x] Server tests: walking 6 m on one Material makes 4 steps; crouching,
      flying and swimming make none; standing still makes none.
- [x] Server tests: a 0.9 m drop makes no `land`, a 1 m drop makes one; the
      Material is the one landed on.
- [x] Steps follow the stride in metres at Voxel sizes 0.5 and 2.0.
- [x] A solid with no Sound set is silent; the stride still advances.

## Comments

- A fall is measured from the feet's highest point since the body last
  stood, not from the landing speed the Vitals use: at the 64 Hz step, v²/2g
  reads a 1 m drop as about 0.95 m. Swimming, flight and a respawn forget the
  height.
- A jump on flat ground rises 1.33 m and so lands with a `land`.
- The solid under the feet is the Voxel a centimetre below the centre of the
  feet; standing out over an edge on a corner of the body reads air and is
  silent.
