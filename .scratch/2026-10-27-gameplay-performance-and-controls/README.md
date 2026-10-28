# Desktop gameplay investigation and agreed design

Status: shared plan confirmed; starting local P0 implementation.

Source baseline: `7907b6de57e337d6257fc5d345c27e098db0b6d1`.
Workspace branch: `tech/gameplay-performance-and-controls-design`.

- [Current spec](spec.md): reconciled decisions from Q1–Q49.
- [Interview record](grilling.md): questions and accepted recommendations.
- [Numbered tickets](issues/): separate PR work units and dependencies.
- [Execution table](execution-plan.md): PR order, tickets, model and effort.
- [Performance evidence](performance.md): measured results and limits.
- [Reproduction](reproduce.md): exploratory profiling patch and scripts.
- [Earlier PR proposals](pr-scopes.md): historical, superseded by the spec.

## Current scope

Desktop only. Improve performance without silently sacrificing quality/reach;
1000+ FPS is a stretch investigation, not a promise. Default display sync,
configurable caps and uncapped benchmarking are distinct from renderer speed.

Remappable controls include optional sprint toggle and timed tap removal,
settings persistence, conflict handling and Apply/Cancel. Catalogue supports
focus/touch assignment, search/filtering, readable scaling and stable browsing.
Controller/touch remain independent PRs, not the first implementation.

Creative organization keeps nine shortcuts with Clear/Swap and per-Game
persistence. Use/Interact is explicit. The first tool is read-only corner
measurement. Owned stacks, consumption, pickup and survival remain deferred.

Add World list/create/seed UI, persistent World changes and paused offline
simulation, 30-second autosave/recovery, one flight/physics Player, a basic
Game-provided blocky character and independent first/third-person views.
The spec records exact cancellation, saving, input and camera behavior.

The user confirmed the consolidated plan after the interview. Start P0
(tickets 11–12) inline, one PR at a time, using separate branches/worktrees.
No parallel coding agents or extra threads are planned. No branch or PR
has been pushed or published. Earlier premature readiness was withdrawn;
actual readiness triage now follows the confirmed spec.

## Existing PR and architectural boundaries

Read-only GitHub inspection found PR 21 already merged externally at
2026-10-27T13:01:36Z, merge 54536199cdd586c84dcf139a360cd02580e32368.
Its head 7907b6d had successful checks. This session did not merge/close it.
The previous 902 passing tests/five ignored measurements are historical;
research builds are recorded separately. Do not reopen approved phase 8.2.

Preserve Engine/Game ownership, Air-only built-ins, Mod ids from 8, Server
World/Player authority, no Client World storage reads, batched WIT and
headless core. Movement/persistence extend the mechanisms deferred by
ADR-0008/0011 through their own reviewed changes.

## Evidence retention

Raw native captures, timings, traces, metadata and exploratory binaries are
in /tmp/tesserae-gameplay-research-gRUJEoDZ, retained for review and outside
git. Product-source profiling changes were restored and the normal dev
executable rebuilt. No other session's measurement artifacts were modified.
The Metal trace failed finalization/export and supplies no GPU timing.
