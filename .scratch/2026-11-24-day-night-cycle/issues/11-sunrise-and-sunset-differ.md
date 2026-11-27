# 11: Sunrise and sunset read differently

Status: done

Spec: [Day and night cycle](../spec.md).

## What to build

Terra's Day curve is mirror-symmetric about noon, so dawn looks like dusk. Make
sunrise feel like the day starting and sunset like it ending, in Terra's data
only (`games/terra/src/sky.rs`); no engine change.

- Sunrise: cooler rose-gold light, a pink-peach horizon fog, less haze; a
  shorter blue hour; full daylight a little sooner.
- Sunset: today's deep orange, a touch redder; a longer golden hour from about
  16:30; a violet blue hour that lingers before night.
- Every ambient key stays at or above the cave floor per channel (the curve's
  validation enforces it).

## Acceptance

- [x] Captures at 05:30, 06:00, 06:30, 07:00 and 16:30, 17:00, 18:00, 18:30
      from the far and near Eyes, approved by the user.
- [x] Terra's tests pass (`cargo test --manifest-path games/terra/Cargo.toml`
      and the sky tests in `tesserae`).

## Comments

Proposed on 2026-11-26 and agreed by the user; the user wants the look shown
before it is settled.

2026-11-27: dawn has a short blue hour (night until 05:00), rose-gold sunrise
light over a pink-peach horizon with less haze, and full day at 08:00. Dusk
has a golden hour from 16:30 deepening at 17:15, a redder sunset at 18:00 and
a violet blue hour from 18:30 to 19:00, night at 20:00. Twelve keys. Captured
at every asked hour from the far and near Eyes; Terra's tests and the sky
tests pass. The user approved the look on 2026-11-27.
