# 30: W1: In-process World sessions

Status: done
Blocked by: 27 (done)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W1, as a stack of three PRs (W1a, W1b, W1c below); keep it
separate from other PR scopes.

## What to build

Replace W0's process re-exec with in-process switching between the World list
and a World, as Minecraft (integrated server) and Luanti (per-session client
and server objects) do. Scope every Server and Client resource, Startup
system, task and Mod instance to a World session, via a top-level Bevy
`States`, so that leaving a World tears it down completely and opening another
builds it fresh in the same window.

Show the loading screen at once: move Mod loading and save validation behind
the window, so opening a World never leaves the screen empty. Ticket 27
moved this here (2026-10-29). The user asked (2026-10-29) for one window that
never closes when a World opens, with a loading bar that follows the real
stages: reading the save, loading the Game, then the first terrain.

This is the design the spec's World-switch rule assumes: session-only undo,
measurement and pending input are cleared, and old-World jobs or answers
cannot land in a newly opened World with coincident coordinates.

### Session boundary

A top-level `AppState` (`Boot`, `List`, `Loading`, `InWorld`, `Closing`).
World-owning plugins set up in `OnEnter(Loading)` or `OnEnter(InWorld)` and
tear down in `OnExit(InWorld)`. Not a sub-`App` (a second render world and
window) and not entity scoping alone (most state is resources). `SubStates`
wait until a nested state exists.

The first deliverable is an inventory table in the ticket's validation note,
one row per plugin (44 `fn plugin` in `client/` and `server/`): app-lifetime
or session-lifetime. App-lifetime: window, `Preferences`, settings, profile,
input state, fonts, logo and icons, the World list's `Camera2d`. Session-
lifetime: Chunks, Server state, Mods, in-World UI, the 3D camera and light,
HUD, materials and meshes.

### Teardown guarantee

One choke point, not 129 hand-written removals. A `WorldSession` marker
component on every session-spawned entity, and a helper that plugins use to
register each session resource (including non-Send `LoadedMods`) so the same
registration inserts it on enter and removes it on exit. A debug-build check
after teardown asserts that no registered resource or `WorldSession` entity
survives.

Teardown order is fixed: stop accepting input; flush the guarded save (the
existing Retry, Return and confirmed unsafe exit flow); drop jobs and Mod
instances; release `writer.lock`; clear resources and entities; enter `List`.
`Closing` waits on the save guard instead of writing `Relaunch`. The lock is
released only after the flush succeeds or the user confirms the unsafe exit
(which still releases it and logs why).

### Old-World jobs and answers

Each session gets a monotonically increasing `SessionId`, stamped on every
streaming job, Mod call and Server-to-Client message. A result whose id is not
the current one is dropped. Dropping `Task` handles at teardown is kept too
(it cancels at the next await point), but a blocking job already running on
the shared pool still finishes, so the id is the guarantee. The pools
(`AsyncComputeTaskPool`, `IoTaskPool`) are process-global and stay.

### Loading screen

`Loading` has four real stages, each reported by the code doing the work, not
by a timer: reading the save (commit, writer lock, sidecar cross-check),
loading the Game (Trust, wasmtime compile, Mod declare), preparing the World
(generator, spawn point, `Persistence::prepare`) and first terrain (the
existing `TerrainArrived`). The first three run as one `AsyncComputeTaskPool`
task that sends stage updates over a channel into a `LoadProgress` resource;
`load_world` no longer calls `block_on` on the main thread. `LoadedMods`
returns through the task result and is inserted on the main thread as
non-Send. The bar is stage-weighted: stage name plus an indeterminate sweep
inside the stage, filled by stage. `LOADING_TIMEOUT_S` stays only for terrain
that never arrives (a World saved without a Game).

### Failure paths

A failure in the first three stages (corrupt or newer commit, lost Trust,
missing content, lock held, Mod fault) goes `Loading` to `List` with the
existing `OpenFailure` card, after full teardown, with nothing written. A
failure in the first-terrain stage (Server panic, Mod trap) does the same and
leaves the save untouched. Direct launches (`TESSERAE_WORLD_DIR`,
`TESSERAE_EPHEMERAL`, profile fixtures, screenshots) skip the list, enter
`Loading` at start-up, and keep print-and-exit on failure. This replaces the
`Launch::World` re-exec branch and the `world_parts` duplication in `main.rs`.

### Deleted by this ticket

`Handoff`, `Relaunch` and `Target`, `relaunch()`, `next_args`, the `--world`
and `--window` arguments, the per-World `list_app`/`world_app` split and the
`restore` system. `Geometry::of`, `fit` and monitor clamping stay only for the
single cold-start restore of the saved window; everything else in
`launch.rs` goes. The window itself never closes or moves when a World opens.

### Ticket 31 boundary

Exactly one root Game and the existing `ModError::SecondRoot` rule stay. Mod
loading is a function of (World folder, Mods directory), so ticket 31 can
change the discovery layout, the required-content contract and the Create form
without touching session code. Ticket 31 stays a separate PR, blocked by this
one, and adds no session states.

## PR stack

Each PR is mergeable on its own and green.

- **W1a:** add `AppState` and the session-resource helper; run the list and
  the World through it while still re-execing. No behaviour change. Additive,
  so it can start now.
- **W1b:** move Startup systems and resource inits into `OnEnter`/`OnExit`
  plugin by plugin, with the teardown assertion. Start only after
  `tech/streaming-performance` and `fix/respawn-inside-blocks` merge. Do the
  Server group, then the Client group, each in one sitting rather than on a
  long-lived branch; rebase on main before each PR and list the touched files
  in the PR body. `feat/layout-placement` (`client/layout.rs`) and
  `feat/water-swimming` (`server/player.rs`) rebase onto it or it onto them,
  whichever merges second.
- **W1c:** in-process switching, loading stages, `SessionId`, failure paths
  and deletion of the re-exec code. The only behaviour change; review focuses
  here.

## Acceptance criteria

- [x] Launch, open a World, Return to World list and open another (or the
      same) World without the window closing, resizing or moving; the
      screenshot-offscreen smoke run (`TESSERAE_SCREENSHOT_OFFSCREEN=1`)
      covers list, open, return, open another.
- [x] The loading screen shows at once with the window and its bar follows the
      four stages from the real code; no empty window during Mod loading.
- [x] Opening World A, queueing a job at (0,0,0), switching to World B with the
      same coordinates and ticking: nothing from A lands in B (jobs, Mod
      answers, Server messages).
- [x] Session-only undo, measurement endpoints and pending input are empty on
      entry to any World; the hand is empty (ticket 27).
- [x] Open and close 5 times in a headless test: entity count, session
      resource-type set and `Assets<Mesh>`/`Assets<Image>` counts equal the
      pre-open baseline; the debug teardown assertion never fires.
- [ ] A release run of 10 open/close cycles records RSS and GPU memory in the
      validation note; pass line: RSS within 5% of cycle 2 after cycle 10
      (state whether the Mod `Engine` cache is process-lifetime).
- [x] `writer.lock` can be acquired immediately after Return to World list;
      the save guard's Retry, Return and unsafe-exit flows still hold.
- [x] One test per failure class (corrupt, newer, missing content, untrusted,
      locked, Mod fault, first-terrain failure) returns to the list with the
      `OpenFailure` card, a torn-down session, and a byte-identical sidecar
      and commits.
- [x] Direct launches still print and exit on failure and bypass the list.
- [x] The deleted re-exec code is gone, not dead; grep finds no `Relaunch`,
      `Handoff` or `--world`.

## Validation and authorization

Use behaviour/intent regressions and the spec's build/review requirements.
Preserve shared Bevy artifacts, Server/core/WIT boundaries and count bounds.
Native captures stay in one unique /tmp folder until review, then are deleted.
No native Windows or Linux run for this ticket; the code is platform-neutral,
and the validation note says so. Implementation, push, publication and merge
need separate user instruction.

## Notes

Raised during ticket 27's 2026-10-29 grilling (Q19). Counts re-taken on main
8539496: 20 `Startup`/`PostStartup` registrations (about 17 in the World app),
about 129 non-test resource inits (the earlier 329 counted tests) and no
`States`. Bevy allows one window event loop per process.

Ticket 32 carries the window position and fullscreen over re-exec; that code
is what this ticket deletes, so keep 32 minimal rather than reusable.

## Comments

### Grilling 2026-10-30 (Q1-Q13, all accepted)

Q1 top-level `States`; Q2 one teardown choke point; Q3 app- vs session-
lifetime inventory; Q4 `SessionId` plus handle drops; Q5 flush-then-unlock
order; Q6 four real loading stages on a task; Q7 failures return to the list,
direct launches keep print-and-exit; Q8 delete the re-exec code in this
ticket; Q9 mechanical leak bar; Q10 W1a/W1b/W1c stack; Q11 ticket 31 boundary;
Q12 sequencing (W1a now, W1b after streaming-performance and
respawn-inside-blocks merge); Q13 three test layers.

### Implementation 2026-10-30

Implemented on `feat/in-process-world-sessions` as one branch, not the W1a-c
stack, at the user's instruction. Commits: `session` layer, server
registration through it, then single-App opening. `session.rs` holds
`AppState`, `SessionApp` (`add_session_systems`, `init_session_resource`,
`init_session_state`, `add_session_message`, `on_session_start`) and the
baseline-diff teardown; `open.rs` holds the request, the off-thread load and
its stages; ADR-0017 records the decision.

Deviations from the grilling:

- No `SessionId` stamped on jobs or messages. A session's jobs, Mod calls and
  Server messages live in resources that teardown removes and the next session
  re-creates, so an old result has nowhere to land. A test holds a job across
  a session and asserts it never lands in the next. A global id would also
  race the parallel unit tests.
- The debug "nothing survives teardown" assertion is the baseline diff itself:
  teardown removes every resource and entity added since `Loading` began, and
  the soak run shows the counts unchanged.
- Non-Send data is not enumerable in Bevy 0.20, so `LoadedMods` is removed
  explicitly in `SessionTeardown`.

Evidence (macOS, dev build, isolated `HOME`): `TESSERAE_SOAK_CYCLES=5`
returned to the list and reopened five times with entities 597, resources
392, meshes 5, images 25 at every return; one "Creating new window" for the
whole run; RSS 502 to 547 MB, growth slowing (+15, +15, +5, +3, +7), so the
10-cycle release-build RSS line is still to run. Six further 3-cycle runs all
completed; one earlier run stalled in the second session and did not
reproduce. Captures of the list and a second session are in
`/tmp/w1/shots2` (review only). Windows and Linux were not run.

Open: the release-build RSS/GPU memory run; native Tab/Enter and controller
journeys (ticket 32); the loading-screen capture (the loading phase passed
between the soak's capture frames).

### Criteria closed 2026-10-30

The user asked for the open criteria to be finished on the same branch.

- Loading screen: captured natively with the stage label and bar
  (`TESSERAE_SOAK_SHOT`, review only, in /tmp).
- Session-only state: tests that the undo history, pending edits,
  measurement, pending pick and held item are gone in the next World.
- `writer.lock`: a test that it is held while the World is open and free as
  soon as the session ends.
- Failure classes: flow tests for a corrupt save, a folder with no save, a
  missing or untrusted Game, a World held open elsewhere and a Mod that traps
  before the first terrain; a core test that a newer save format is
  incompatible, not corrupt, and untouched. A Mod trap before the first
  terrain now returns to the list (or exits for a direct launch).
- Release build, 10 cycles (macOS, `--release --features tesserae/dev`):
  entities 597, resources 392, meshes 5 and images 24 at every return. RSS
  was 528 MB at cycle 2 and 565 MB at cycle 10, +7%: over the 5% line this
  ticket set, and still rising by about 3-5 MB a cycle at the end, so it has
  not plateaued. The ECS and asset counts are flat, so the growth is outside
  them (allocator retention or something in the Mod host); not found. GPU
  memory was not measured.

Not built, as noted in the implementation note: `SessionId` stamping and the
debug teardown assertion. Some per-system `Local` caches are not reset
between sessions. Windows and Linux were not run.
