# 28: The bed bends through the surface

Status: done
Blocked by: 27

Spec: [Water that looks natural](../spec.md). From the user's review.

## What to build

Near the camera, the bed seen through the water bends with the ripples, the
strongest cue that clear water is there. The water is drawn blended over the
frame, so this needs a copy of the scene behind it, which Bevy's transmission
pass gives, and moves water into that phase. Prototype it, time it at the
water fixtures against the +0.5 ms budget, and show the user before keeping
it.

## Acceptance criteria

- [ ] A prototype's captures and GPU timing, shown to the user, and their
      decision recorded here.

## Comments
- 2026-11-27, the prototype. The blended layer moves into Bevy's transmissive
  pass while `TESSERAE_DIAG_WATER_REFRACTION` is set (`--switches
  water-refraction` in `scripts/profile-frames`), under `dev` and
  `profiling` only. Within 40 m, the shader adds (1 - alpha) times the
  difference between the scene behind, bent a quarter of the ripple's tilt
  over up to 2 m of depth, and the scene straight behind; the blend and fog
  are unchanged. Captures `/tmp/tesserae-water-look-captures/28-proto/`.

  GPU p50, six rounds each, off then on: lake 4.01 to 4.45 ms (+0.44), river
  13.96 to 14.07 (+0.11), coast 3.34 to 3.47 (+0.14). The lake is already
  +0.42 against `main`, so on by default it would be about +0.86, past the
  budget. Left off; the user decides between a High-quality setting and
  dropping it. Risks if kept: glass behind water drops out of the bent view,
  and Transparent-phase effects drawn after it sort over water.

- 2026-11-27: the user chose a player setting. Display → Image quality →
  Water refraction, off by default, approved in the mockup first; it adds the
  camera's copy of the scene and makes water transmissive together, and turns
  both off again live. `TESSERAE_DIAG_WATER_REFRACTION` still forces it on for
  `profile-frames`. Captures `/tmp/tesserae-water-look-captures/toggle/`.
