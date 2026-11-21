# 03: No cutout casters in the far cascades (1a)

Status: done
Blocked by: 01

Spec: [Shadows and culling](../spec.md).

## What to build

Cutout layers (foliage) are the casters that run a fragment shader in the shadow
pass, once per cascade that lists them.

- A system after Bevy's directional caster visibility removes every cutout Chunk
  layer from the last cascade's visible list, from 61 m: cascade 3 of Bevy's
  four, then cascade 2 of 06's three. Chosen on captures over cascades 2 and 3
  of four, from 25 m. Nearer cascades are untouched.
- Which cascades are far is one named constant, revisited by ticket 06.

## Acceptance criteria

- [x] A Bevy app test: a cutout layer in a far cascade is not in its list, and
      is in a near cascade's; opaque layers are in both.
- [x] Gated by 01's method, the shadow span as its target; numbers in Comments.
- [x] Before and after captures of `flight`, `stationary` and `hidden-hill`
      shown to you; lands only on your approval.

## Comments

- Built in `client/chunk_view.rs`: cutout layers spawn with a
  `NearCascadesOnly` marker (`Draw::show` takes a `Casts` choice in place of a
  bool), and `drop_cutout_from_far_cascades` runs in `PostUpdate` after
  `check_dir_light_mesh_visibility`, removing them from every cascade from
  `FIRST_FAR_CASCADE = 2` on. Avatars, drops and opaque layers are untouched.
- Test `leaves_cast_in_the_two_near_cascades_only_and_ground_in_all_four` runs
  Bevy's own caster check under four unbounded cascades, so it also catches a
  Bevy upgrade that moves the check. The seam is listed in DESIGN.md's
  `bevy_pbr` friction points.

Gate (2026-11-21), against 02, release `profiling` builds, load 5 to 6, built
at 25 m: passes. The shadow span barely moved (0.164 → 0.155 ms in `flight`);
the saving shows in the GPU frame, beyond run spread in every fixture, and in
frame p50 in both timing rounds.

| p50 ms (02 → 03 at 25 m) | GPU frame | frame, round 1 | frame, round 2 |
| --- | --- | --- | --- |
| flight | 4.11 → 3.37 | 3.43 → 3.00 | 3.90 → 3.04 |
| active | 4.62 → 3.51 | 3.97 → 3.27 | 3.89 → 3.43 |
| transparency | 4.54 → 3.51 | 4.13 → 3.13 | 3.94 → 3.30 |
| stationary | 4.26 → 2.99 | 3.27 → 2.74 | 3.50 → 2.78 |
| sprint-flight | 3.61 → 3.45 | 3.81 → 3.27 | 4.21 → 4.03 |

Captures at 25 m against 02: `hidden-hill` identical; `stationary` 197,000
pixels and `flight` 610,000 changed (two runs of 02 differ by 0 and 91). Trees
past 25 m lose their leaf self-shadowing and the ground under them its dapple,
which from the air is the whole canopy.

Reach, on the branch with 05 (2026-11-21): leaves stopping at 25 m, at 61 m
and never, run back to back (25-61-never-never-61-25, timing only, pooled p50
in ms):

| fixture | never | 61 m | 25 m |
| --- | --- | --- | --- |
| flight | 3.57 | 3.24 | 3.13 |
| active | 4.10 | 3.42 | 3.29 |
| transparency | 4.14 | 3.37 | 3.32 |
| stationary | 3.35 | 2.78 | 2.77 |

61 m keeps 75 to 99% of the saving and halves `flight`'s changed pixels
(335,000), keeping the canopy below the camera shaded; `stationary`'s treeline
is past 61 m, so it changes as at 25 m. p95 is flat across the three. Approved
at 61 m: `FIRST_CASCADE_WITHOUT_CUTOUT = 3`.
