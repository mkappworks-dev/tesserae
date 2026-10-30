# Sequential PR execution

Updated 2026-10-30 against main `ef6835d`. Feature scope, dependencies and
acceptance criteria live in spec.md and the numbered tickets. Numbers below
are sequence positions, not GitHub PR numbers; the GitHub number is in the
last column.

Model: Claude Sonnet 5.5 at Medium effort by default, High for the Server
authority, persistence, physics and session-lifecycle scopes. These are
starting points based on task risk, not Tesserae benchmark results. Raise
effort only for a concrete problem: repeated failed fixes, ambiguous
ownership, concurrency or difficult state transitions.

## Merged

| #   | Planned PR                                          | Tickets | GitHub    |
| --- | --------------------------------------------------- | ------- | --------- |
| 1   | Profiling and release benchmarks                    | 11-12   | #22       |
| 2   | Input, actions, controls and display settings       | 01-04, 28 | #24     |
| 3   | Catalogue and creative scope, hotbar retry fix      | 05-06, 17 | #25     |
| 4   | Save schema, lifecycle and durable World state      | 22-23   | #23       |
| 5   | Flight and physics movement                         | 24      | #26       |
| 6   | Character and third-person camera                   | 25-26   | #29       |
| 7   | World list, creation and seed selection             | 27      | #30       |
| 8   | Explicit Use and a first measuring tool             | 20-21   | #32       |
| 9   | Controller                                          | 07      | #28       |
| 10  | Desktop touch contacts and overlay                  | 08-10   | #38       |
| 11  | Repeat place and remove while held                  | 29      | #31       |
| 12  | Water immersion: view, then movement                | 33      | #34, #37  |
| 13  | World list window and controller follow-ups         | 32      | #33       |
| 14  | In-process World sessions                           | 30      | #39       |

Also merged outside the ticket plan: #35 (lift a physics respawn out of
Voxels) and #36 (CAD-style layout placement with live preview).

## Open

| #   | Planned PR                                          | Tickets | Status       | Model, effort |
| --- | --------------------------------------------------- | ------- | ------------ | ------------- |
| 15  | Multiple installed Games and per-World Mod sets (G0) | 31     | needs-triage | Sonnet, High  |
| 16  | World session memory growth (W1.1)                  | 34      | needs-triage | Sonnet, Medium |

Ticket 31 needs a grill pass before code. Ticket 34 is blocked only by the
now-merged ticket 30 and should be triaged first: it decides whether the
roughly 3 MB per World session is a leak or an accepted cost.

## Conditional experiment backlog

Tickets 13–16 are hypotheses, not scheduled implementation PRs. P0 did not
establish worthwhile gains for any of them. They block nothing, and no
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
deferred. They are separate from the conditional performance experiments and
become DESIGN.md step 13 (Inventory).
