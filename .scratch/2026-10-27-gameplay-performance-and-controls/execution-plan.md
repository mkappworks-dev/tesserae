# Sequential PR execution and model recommendations

Updated 2026-10-28. This adds execution guidance to the confirmed spec;
feature scope, dependencies and acceptance criteria remain in spec.md and
the numbered tickets. Numbers below identify sequence positions, not GitHub
PR numbers. P0 merged as [PR 22](https://github.com/mkappworks-dev/tesserae/pull/22);
the condensed sequence below supersedes the earlier sixteen-position plan.

Default: GPT-6.1 Terra (`gpt-6.1-terra`), Medium reasoning effort.
Use GPT-6.1 Sol (`gpt-6.1-sol`) with High effort for the investigation,
persistence and physics scopes marked below. These are starting-point
recommendations based on task risk, not Tesserae model benchmark results.
The PR 1 session was launched with Sol High.

| # | Planned PR | Tickets | Model | Effort |
| --- | --- | --- | --- | --- |
| 1 | Profiling and release benchmarks (merged as #22) | 11–12 | Sol | High |
| 2 | Input, actions, controls and display settings | 01–04, 28 | Terra | Medium |
| 3 | Catalogue/creative scope completed against accepted main | 05–06, 17 | Terra | Medium |
| 4 | Save schema, lifecycle and durable World state | 22–23 | Sol | High |
| 5 | Flight and physics movement | 24 | Sol | High |
| 6 | Character and first/third-person cameras | 25–26 | Terra | Medium |
| 7 | World selection and seeded creation | 27 | Terra | Medium |
| 8 | Use interaction and corner measurement | 20–21 | Terra | Medium |
| 9 | Controller | 07 | Terra | Medium |
| 10 | Desktop touch | 08–10 | Terra | Medium |

PR 2 implementation is complete. The user's 2026-10-29 instruction authorizes
reconciling local main, rebasing this branch, pushing it and opening a PR.
No merge, parallel agents or automatic later sessions are authorized.

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

## Unscheduled World list work

Ticket 30 (W1, in-process World sessions, including a loading screen before
Mod loading) is a needs-triage backlog item with no sequence slot. It
touches nearly every Server and Client plugin, so schedule it after PRs 6
and 8. Ticket 32 (W0.1: window position and native fullscreen across a
restart, the World screens' native keyboard check and their controller
navigation) is a small separate PR that may land before 30. Ticket 31 (G0,
multiple installed Games) is also unscheduled.

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

## Catalogue worktree outcome

The user requested reverting all PR 3 menu UI changes. Menu, settings,
profile UI and input behavior match merged main `4b86e43`. The user then
accepted that existing behavior as the completed scope of tickets 05, 06 and
17. All three are marked done; their expanded, unimplemented requirements
are removed rather than carried forward as pending work. Only a non-UI
hotbar failed-write retry correction remains as new source in PR #25.

The user subsequently authorized consolidating the remaining changes into
one commit, pushing the branch and opening a focused bug-fix PR. Merge is
not authorized. See [current validation](pr3-implementation-validation.md).

## PR 5 implementation

The user's 2026-10-29 instruction authorizes ticket 24 in its existing
worktree against merged controls #24 and persistence #23. Historical
readiness gates are superseded. Scope and review evidence are recorded in
[PR 5 validation](pr5-movement-validation.md). The user later authorized
rebasing onto main, pushing and opening the PR. Merge is not authorized.
