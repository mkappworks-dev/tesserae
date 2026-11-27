# 31: Falls reach their pool

Status: done
Blocked by: 22

Spec: [Water that looks natural](../spec.md), Rivers. From the user's review:
some waterfalls stop on blocks before they reach the bottom level.

## What to build

A Fall's water runs from its lip all the way to the pool below. Where ground or
a ledge stands between the lip and the pool, either the Fall steps down it as a
cascade or the ground is cut away, so no Fall ends in mid air on rock.

## Acceptance criteria

- [ ] A test over every Fall in the generated boxes: no solid Voxel between a
      Fall's sheet and its pool.
- [x] The cause named in Comments.
- [x] Captures of Falls that stopped before, shown to the user.

## Comments

- 2026-11-27: a tall Fall's face is a short staircase of ledges (3.5 in 1), and
  its pool lies up to three columns past the lip; the one sheet hung on the
  lip's face, and the first ledge hid the rest of it inside rock. `Fall`
  carries its `reach`, and each ledge between lip and pool now gets a film over
  its top and a sheet down its face to the next ledge or the pool, drawn from
  Chunks on either side of the lip. Test `a_fall_runs_down_every_ledge_to_its_pool`
  through the mesher; the generated boxes' Falls are checked as before by
  `a_fall_runs_over_every_lip_with_a_lower_pool_past_it_and_nowhere_else`.
  `/tmp/tesserae-water-look-captures/31-falls/fall-front.png`, shown to the
  user.
