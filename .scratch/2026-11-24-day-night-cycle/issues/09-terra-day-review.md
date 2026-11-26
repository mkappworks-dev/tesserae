# 09: Terra's day, reviewed whole

Status: ready-for-agent
Blocked by: 03, 05, 06, 07, 08

Spec: [Day and night cycle](../spec.md).

## What to build

The whole day on Terra, judged together once every part has landed, and the
records brought up to date.

- Captures at every baseline Eye at noon, dusk, full-moon and new-moon
  midnight, and one full day watched at speed, reviewed by the user; any last
  Day curve or cloud tuning lands here.
- The sky and the Cloud layer together at most 0.3 ms GPU frame time over
  `main` on `flight`, at noon and midnight.
- DESIGN.md's step 9 note records the cycle as done, with the measurement.

## Acceptance

- [ ] The user approved the full day.
- [ ] Frame time within budget, recorded in Comments.
- [ ] DESIGN.md updated; the spec's Status set to done.
