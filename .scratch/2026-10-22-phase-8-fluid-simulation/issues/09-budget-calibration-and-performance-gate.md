# 09: Calibrate responsive fluid and fire budgets

Status: ready-for-agent
Blocked by: 08

Spec: [decision 11 and measurement policy](../spec.md).

## What to build

Choose evidence-backed defaults that make ordinary fluid/fire interactions
visible while preserving player Edits, Mod fairness, streaming, and bounded
tick work. Resolve the named relight performance question explicitly.

## Acceptance criteria

- [ ] Native benches cover idle/dry terrain, uniform lakes, flow/recession,
  waterlogging-heavy geometry, reactions, ignition/burning/extinguishing, and
  saturated continuation work. Track CPU, operations, and retained memory.
- [ ] Controlled/live scenarios include a 16 m waterfall, a flat basin wider
  than water reach, four separated active outlets, a 16 m fuel chain with a
  barrier and extinguishing water, and a saturated frontier.
- [ ] Use equal physical fixtures at 1.0, 0.5, and 0.25 m Voxel sizes. Report
  logical progress, source-to-visible latency, mutation/mesh progress, backlog,
  and complete Server tick distributions beside CPU and load figures.
- [ ] Compare affected native hot paths with the pre-change evidence from
  ticket 01. CPU comparisons use equal work and interleaved orders; live
  unequal-presentation CPU is labeled as observation, not throughput.
- [ ] Record uptime before and after every process and load beside every
  figure. Use unique paths, retain raw records, and discard distorted pairs
  from conclusions without concealing them.
- [ ] Set count quotas from repeated quiet-load evidence, using 15.625 ms at
  64 Hz as the full-tick reference. Report control and fluid/fire distributions
  without claiming a universal wall-time bound from a count budget.
- [ ] Show that ordinary interactions progress usefully, settled state is
  idle, saturated work stays bounded/fair, and continuous changes still draw.
  Do not preserve responsiveness by accepting unusably slow ordinary flow.
- [ ] Evaluate the repeated incremental relight cost. A general bulk relight
  rewrite stays a named follow-up; if required to pass the accepted scenarios,
  present evidence and obtain an explicit prerequisite/scope revision before
  adding it or calling the phase complete.
- [ ] Do not raise the phase 7 two-Voxel Mod drain cap or alter security
  patches as an implicit performance adjustment.

## Verification and completion

Native measurements, integration runs, and conclusions belong in Comments.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- This is an integration/measurement gate. A correct but stalled waterfall,
  unmeasured tiny quota, or skipped relight issue does not satisfy it.
