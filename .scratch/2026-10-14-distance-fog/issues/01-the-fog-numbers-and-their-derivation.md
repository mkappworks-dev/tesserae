# The fog numbers and their derivation

Status: done
Blocked by: -

Spec decisions 2 and 7.

## What to build

A new `crates/tesserae/src/render/fog.rs`, `pub mod fog;` in `render/mod.rs`,
and nothing attached to a camera yet.

- `FogSettings { color: [f32; 3], reach_opacity: f32 }`, a `Resource`, `Clone`,
  `Copy`, `Debug`, `PartialEq`, with a `Default`: `color` is sRGB (43, 44, 47)
  over 255 (today's `ClearColor`, which the next ticket replaces), and
  `reach_opacity` is 0.85.
- A function from `&FogSettings`, `view_distance_m` and `lod_distance_m` to
  Bevy's `DistanceFog`. The reach is `max(lod_distance_m, view_distance_m)` and
  the falloff is `FogFalloff::ExponentialSquared` with
  `density = sqrt(-ln(1 - reach_opacity)) / reach_m`. Guard the ends so a
  `reach_opacity` of 0 or 1 or a non-positive reach cannot give a NaN, an
  infinity or a divide by zero; say in the doc what each end means.
- The module's doc comment says what the numbers are (colour and opacity at the
  reach, both plain data) and that the density is derived, so a Look pack in
  step 9 gives two numbers that are right at any reach.

No system, no plugin, no camera, no shader.

## Acceptance criteria

Unit tests, in `fog.rs`, each naming the claim:

- The opacity at the reach, `1 - exp(-(density * reach)^2)`, equals
  `reach_opacity` to within float error, for several reaches and several
  opacities.
- Opacity is monotone in distance, 0 at 0.
- At the defaults, level 0's edge (192 m) is under 1% and the join of levels 1
  and 2 (384 m) is under 2%, so the full-detail rings are not hazed. (These are
  the spec's figures; a change to the defaults that breaks them is deliberate
  and edits the test with the reason.)
- The fog is built from the two distances and the settings alone: there is no
  Voxel size in the signature, and a doc line says that is the point
  (ADR-0003).
- A World with no coarse levels (`lod_distance_m <= view_distance_m`) fogs to
  `reach_opacity` at `view_distance_m`.
- `reach_opacity` of 0 gives no fog, 1 and a reach of 0 give finite numbers.
- Mutation-check the density formula (drop the square root, swap the reach for
  `view_distance_m`) and say which test killed each.
- `./scripts/cargo-ci` passes.
- `Status: done` is set in the commit that finishes this ticket.

## Comments

Mutation-checked: dropping the square root fails the reach, no-coarse-levels and
unhazed-rings tests; using `view_distance_m` as the reach fails the reach,
unhazed-rings and monotone tests (the no-coarse-levels test passes by design,
since the two reaches coincide there). Removing the opacity ceiling or the reach
floor fails the finite-numbers test.

The module carries `#![cfg_attr(not(test), expect(dead_code))]` because nothing
uses the fog until ticket 02 attaches it to a camera; ticket 02 deletes that
line.
