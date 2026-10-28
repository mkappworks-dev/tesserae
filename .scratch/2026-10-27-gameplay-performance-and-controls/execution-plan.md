# Sequential PR execution and model recommendations

Updated 2026-10-28. This adds execution guidance to the confirmed spec;
feature scope, dependencies and acceptance criteria remain in spec.md and
the numbered tickets. PR titles below describe planned scopes, not published
GitHub PRs.

Default: GPT-6.1 Terra (`gpt-6.1-terra`), Medium reasoning effort.
Use GPT-6.1 Sol (`gpt-6.1-sol`) with High effort for the investigation,
persistence and physics scopes marked below. These are starting-point
recommendations based on task risk, not Tesserae model benchmark results.
The PR 1 session was launched with Sol High.

| # | Planned PR | Tickets | Model | Effort |
| --- | --- | --- | --- | --- |
| 1 | Frame profiling and release benchmarks | 11–12 | Sol | High |
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

Each PR runs sequentially in its own branch/worktree. The user requested a
new session and worktree for PR 1; it runs on
`tech/frame-profiling-benchmarks`. That request supersedes the earlier
same-session preference for PR 1. It does not authorize parallel agents or
automatically starting all later sessions. No push or publication is
authorized.

The following remain conditional or deferred; they are outside the sixteen
defined implementation scopes above.

| Status | Separate PR scope | Tickets | Model | Effort |
| --- | --- | --- | --- | --- |
| Evidence gate | Static atlas optimization | 13 | Terra | Medium |
| Evidence gate | Quiet UI optimization | 14 | Terra | Medium |
| Evidence gate | Per-view render optimization | 15 | Sol | High |
| Evidence gate | Emitter relight optimization | 16 | Sol | High |
| Deferred | Owned inventory transactions | 18 | Decide later | — |
| Deferred | Owned resource persistence | 19 | Decide later | — |

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
