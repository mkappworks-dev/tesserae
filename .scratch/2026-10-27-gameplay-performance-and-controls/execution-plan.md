# Sequential PR execution and model recommendations

Updated 2026-10-28. This adds execution guidance to the confirmed spec;
feature scope, dependencies and acceptance criteria remain in spec.md and
the numbered tickets. Numbers below identify sequence positions, not GitHub
PR numbers. P0 merged as [PR 22](https://github.com/mkappworks-dev/tesserae/pull/22);
positions 2–16 remain planned scopes.

Default: GPT-6.1 Terra (`gpt-6.1-terra`), Medium reasoning effort.
Use GPT-6.1 Sol (`gpt-6.1-sol`) with High effort for the investigation,
persistence and physics scopes marked below. These are starting-point
recommendations based on task risk, not Tesserae model benchmark results.
The PR 1 session was launched with Sol High.

| # | Planned PR | Tickets | Model | Effort |
| --- | --- | --- | --- | --- |
| 1 | Frame profiling and release benchmarks (merged) | 11–12 | Sol | High |
| 2 | Input ownership and semantic actions | 01–02 | Terra | Medium |
| 3 | Remapping, sprint toggle and timed tap removal | 03–04 | Terra | Medium |
| 4 | Display sync and FPS limit settings | 28 | Terra | Medium |
| 5 | Searchable catalogue and accessible navigation | 05–06 | Terra | Medium |
| 6 | Creative organization and remembered shortcuts | 17 | Terra | Medium |
| 7 | World identity, save schema and lifecycle | 22 | Sol | High |
| 8 | Durable World changes and simulation state | 23 | Sol | High |
| 9 | Free-flight and physics movement modes | 24 | Sol | High |
| 10 | Game character rendering and animation | 25 | Terra | Medium |
| 11 | First-person and third-person views | 26 | Terra | Medium |
| 12 | World selection and seeded creation | 27 | Terra | Medium |
| 13 | Use interaction and read-only tool mode | 20 | Terra | Medium |
| 14 | Corner measurement tool | 21 | Terra | Medium |
| 15 | Configurable controller support | 07 | Terra | Medium |
| 16 | Desktop touch controls and preferences | 08–10 | Terra | Medium |

Each PR runs sequentially in its own branch/worktree. P0 is complete;
PR 2 (tickets 01–02) is next. Starting later work, extra threads, publication
and merges require their own applicable user authorization. No parallel
coding agents are planned.

## Conditional experiment backlog

Tickets 13–16 are hypotheses, not scheduled implementation PRs. P0 did not
establish worthwhile gains for any of them. They do not block PR 2, and no
PR title or implementation commitment is reserved. Each ticket records the
specific missing evidence. Use stable matched runs at unchanged quality,
measure an effect above overhead/variation, and preserve output and tails.
Promote only a demonstrated candidate; defer or close a failed experiment.

- Ticket 13: investigate recurring atlas work and readiness-safe invalidation.
- Ticket 14: attribute idle UI/system/layout work before adding caches.
- Ticket 15: obtain valid per-view CPU/GPU attribution before choosing a change.
- Ticket 16: attribute emitter relight in targeted simulation stress fixtures
  and prove a bounded equivalent algorithm before implementation.

See [P0 validation](implementation-validation.md) for qualified measurements
and evidence gates. Whole-schedule timings do not establish per-view or GPU
bottlenecks; the broad noisy series cannot justify small gains.

## Deferred product work

Tickets 18–19, owned inventory transactions and resource persistence, remain
deferred. They are separate from the conditional performance experiments.

For Terra Medium scopes, increase effort to High only when a concrete
problem warrants it: repeated failed fixes, ambiguous ownership, concurrency
or difficult state transitions. Reconsider the model if that still does not
resolve the issue. Do not make Extra high, Max or Ultra the default.
These recommendations do not change an active session's settings or
require extra review agents.

The local T3 catalog confirms both models and Medium/High settings are
available. Official guidance recommends evaluating reasoning effort against
quality, latency and cost rather than assuming more effort is always better:
[Reasoning guide](https://developers.openai.com/api/docs/guides/reasoning).
