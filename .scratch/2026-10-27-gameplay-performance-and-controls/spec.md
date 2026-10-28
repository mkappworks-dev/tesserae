# Desktop gameplay, World persistence and measured performance

Status: shared understanding confirmed; local P0 implementation authorized.

Source baseline: `7907b6de57e337d6257fc5d345c27e098db0b6d1`.
All Q1–Q49 recommendations were accepted during the grilling interview.
[grilling.md](grilling.md) retains the questions and acceptance history.
This document reconciles those decisions; it is the current product spec.
[pr-scopes.md](pr-scopes.md) is historical research, not an alternate spec.

## Scope and authorization

Desktop only: macOS, Linux and Windows. Desktop touch is an input capability,
not mobile/web/console support and not the selected first implementation.
Keep independent features in separate PRs. Begin with performance baseline
and profiling, then controls/UI; subsequent scopes follow their dependencies.

The user requested implementation but correctly identified missing grilling.
Earlier readiness labels were premature. The interview is complete and
the user confirmed shared understanding with
ok after the consolidated plan. Local implementation begins with P0. No push,
PR publication or merge is authorized. PR 21 was merged externally and is
not to be reopened or extended. Its approved phase-8.2 spec stays intact.

## Performance and presentation

Preserve current visual quality and view distance for normal-play performance
improvements. Reduced-quality maximum-FPS modes are optional, explicitly
labeled and measured separately. 1000+ uncapped FPS is a stretch investigation,
not a promise or a mandatory release gate. The Minecraft Vulkan example is
inspiration; its unknown hardware/settings do not establish a fair baseline.

Normal play defaults to display-synced rendering. Expose configurable FPS
caps and an uncapped option for benchmarking, without silently changing
quality, reach or simulation rate. Measure release builds separately from
dev and record actual present mode, focus and target resolution.

The historical fixture has a 6.99 ms median of run medians and 12.66 ms
median p99 at 1080p offscreen in dev; the reduced-quality probe is 2.45 ms.
GPU pass cost and native presentation remain unmeasured. The earlier 144 Hz
proposal is not an agreed ceiling or numeric acceptance gate. Establish
matched release evidence before choosing numerical optimization gates.
See [performance.md](performance.md) for raw methodology and qualifications.

Profiling must separate startup, stationary play, flight/streaming, dense
transparency and active fluid/fire. Record CPU/wall, extraction/render,
Server/upload/sort, GPU/present availability and simulation progress.
Use real unclamped frame timestamps and aligned frame ids. Report
p50/p95/p99/max over at least three interleaved runs. Profiler overhead and
quality changes cannot be pooled into equal-quality improvements.

Optimize only measured work in narrow PRs. Static atlas rendering and quiet
UI invalidation are candidates, not promised gains. Per-view render changes
and emitter relight require evidence/proofs before implementation. Preserve
simulation bounds, Mod drain limits and output equivalence.

## Input ownership and remapping

Resolve physical input once into semantic actions under explicit gameplay,
catalogue, settings capture, text-entry and unfocused contexts. Specific
modifier chords consume triggers before less-specific actions. UI owns its
inputs; entering/leaving UI or regaining focus cannot manufacture a press.
Cancel active axes, partial actions and stale sampling on context/device
changes. Returning to gameplay requires fresh destructive activation.

Rebind all current gameplay and menu/help actions with two alternatives per
action for keyboard/mouse. Reserve Escape for cancelling binding capture.
Warn on conflicts in overlapping contexts and offer replace or cancel;
never silently fire competing actions. Capture starts after its opening
input is released. Respect both modifier sides and platform undo/redo.

Preserve current defaults: hold removal, hold sprint, right place/ignite,
X empty hand, middle/P sampling and fire > solid > source removal priority.
Keep Server aim/history guards and one removal per activation.

Add optional behavior preferences:

- Tap removal starts the existing Material-timed action without holding,
  completes once, and a second tap cancels. It is not instant removal.
- Tap progress stays on its original target. Target change, lost reach or
  invalid/stale Server aim cancels; progress never transfers.
- Place/ignite, sample, hand/slot/Shape/orientation changes and undo/redo
  cancel pending tap removal. Perform the new valid action normally, with
  no automatic removal restart.
- Toggle sprint stays enabled until pressed again, including across brief
  stops. Menu entry, focus loss and respawn clear it. Show an indicator.
- Include mouse sensitivity, invert Y and hotbar scroll direction/sensitivity.

Store a versioned user profile across Worlds/restarts with individual and
full reset, atomic writes, corrupt-data recovery and new-action defaults.
Games do not silently overwrite bindings. Stable action/key ids and readable
layout-aware labels are distinct. Full localization is separate work.

Controls settings are accessible from catalogue and help, without requiring
another default shortcut. Apply activates/saves changes; Cancel preserves
old settings. Back/Escape with unsaved edits offers Apply, Discard or Stay.
Instructions/cues reflect current bindings and behavior preferences.

## Catalogue and readable UI

Preserve existing mouse quick-hold, drag/drop and number shortcuts. Add
visible Hold and Assign to slot operations for keyboard/touch selections.
Provide focus navigation, activation and cancellation without virtual mouse
input. Restore focus after settings/dialogs. Hover/drag cannot be required.

Search declared Material/Shape names. Filters are All, Solids, Fluids, Fire
and Mod. New Game-authored categories are later content-contract work.
Remember search, filters, scroll and selection within the same World session;
opening another World resets them.

An explicit Material selection keeps its Shape panel while the pointer
passes other tiles. Hover may highlight/preview without replacing it.
Keyboard/controller focus selects the corresponding panel. Keep source
entities stable during drag and focus stable across rebuilds.

Show readable short names on tiles and full namespaced identity in focused
details/tooltips. Visibly distinguish duplicate names from different Mods.
Support UI scale and label sizing while preserving the default appearance.
The quick-slot footer stays fixed; use horizontal scrolling rather than
unreachable slots in narrow/enlarged layouts. Use text/icons as well as colour
for validity, selection and refusals.

Engineering validation matrix: 1024 x 640 logical at 100%, and 1280 x 720 /
1920 x 1080 at 100/150/200% UI scale. Touch targets are at least 44 logical
pixels with spacing. Keep all required actions reachable without clipping.
Catalogue/settings leave Server simulation running. Pausing is separate work.

## Desktop controller and touch

Controller sticks directly move/look during play; menus use focus navigation,
not a virtual mouse cursor. Prompts follow deliberate last-used input, not
drift. Controller bindings, sensitivity, invert Y and deadzones are separately
configurable without replacing keyboard/mouse preferences. Apply trigger
hysteresis and cancellation on disconnect. Validate default mappings on real
hardware before claiming support; injected tests are not physical validation.

Touch owns pointer ids from start to end/cancel. Support simultaneous movement,
look and explicit actions, with no gesture transfer from UI to gameplay.
Synthetic mouse events must not duplicate touch actions; real mouse input
works after touch finishes. Do not implement touch by mutating mouse state.

Intentional touch use reveals an optional overlay: left movement, right look
and separate place/remove controls. Keep keyboard/mouse usable; do not leave
a permanent touch overlay in ordinary mouse play. Offer normal/mirrored
layouts, size and opacity. A freeform HUD editor is deferred. Preserve the
agreed tap/hold semantics and touch-safe catalogue operations. Native desktop
touch hardware is required for support acceptance.

## Creative organization and interaction

The first inventory PR is unlimited creative selection/organization, not
finite owned stacks. Keep nine shortcuts plus temporary outside selection;
no bag grid or extra hotbar pages. Add explicit Clear slot and Swap actions.
Clearing removes a shortcut, not content; X empties only the hand. Dropping
a drag outside a valid slot cancels it rather than implicitly deleting it.

Save arrangements per Game using stable Material/Shape names and Orientation.
Restore across restarts; missing content is visibly empty, not a different
item at a reused id. Controls remain global per user. Temporary outside-held
selection is not persisted. These shortcuts are not owned resource saves.

Use/Interact is a dedicated remappable action. Right click stays place/ignite;
there is no use-before-place or automatic fallback. Empty hand, sampling,
pickup and use remain distinct. Finite stacks, consumption, physical pickup,
durability and survival undo are later policies, not approved by this spec.

## Measurement tool

The first tool is Game-declared read-only measurement, not mining or a fluid
container. In measurement mode, Use selects two Server-resolved endpoints;
player place/remove actions are disabled until leaving/cancelling the mode.
This does not pause simulation or freeze other World changes.

Snap endpoints to Voxel/Shape corners with a visible preview. Show a line,
straight-line distance and X/Y/Z differences in metres and Voxel units.
Each endpoint must be selected within normal Edit reach; the Player can move
between selections to measure a larger span. Keep one completed measurement
for the World session, replaced by the next, with Clear/Cancel. Saved marker
collections and unrestricted free-point drawing are deferred.

## World creation, seed and persistence

Provide a World list with Create, Open and Rename. New World asks for name,
installed Game (or empty World) and optional seed. Mod download/management
is separate; keep explicit Trust checks before running Mod code.

Blank seed chooses a random seed; Randomize can choose another. Accept
numbers and words, with stable documented text conversion. Display/copy the
resolved numeric seed and save it with World settings. Matching seed,
Game/content, generator version, parameters and Voxel size reproduce starting
terrain; distinct seeds need not produce distinct terrain. Changing an
existing World's seed in place is not supported. Different Worlds may share
a name/seed without sharing save identity or overwriting each other.

Use the current terrain generator initially; new biomes/terrain styles are
separate work. Audit full-range seed arithmetic, including the known fbm2
overflow boundary, before exposing unrestricted random/custom input. Keep
benchmark seeds fixed, recorded and separate from random World creation.

Save changed Server-owned World state: solids/removals and fluid/fire, whether
changed by Player, Mod or simulation. Persist across eviction/reload and
World reopen. Persist seed, Voxel size, generator parameters/version and
stable content identity. Rebuild derived meshes/light and coarse levels from
saved authoritative data as appropriate; don't restore generated terrain over
saved edits. Natural LOD feature loss remains the existing geometric policy.

Autosave every 30 seconds; save before evicting changed Chunks and on Save &
Quit. Snapshot/serialize off-thread where appropriate; dirty versions and
landing/identity guards remain Server-owned. Never discard a newer dirty
change because an earlier snapshot finished. Keep a recoverable last completed
save. A crash may lose changes since that save; zero-loss recovery is not
promised. Save failure/unsaved state is visible. Failed Save & Quit keeps the
World open with Retry/Return to game. Explicit exit without saving requires
confirmation, never a falsely successful save report.

Pause fluid/fire for unloaded Chunks or closed Worlds; resume saved state and
relevant timers when loaded, without offline catch-up bursts. Saving must
preserve observable simulation state, not merely the rendered geometry.
Do not blindly rerun new-World Game setup over restored data. Distinguish new
and reopened lifecycle through the Game boundary; this spec does not imply
snapshotting an entire WASM process or introducing network transport.

Missing required Game/Mod content or incompatible saves stop normal loading
with an explanation. Do not silently substitute Air or overwrite the save.
Migration and recovery must be explicit and preserve the last committed data.
Owned inventory persistence remains a separate later scope. World switches
clear session-only undo, measurement and pending input; old-World jobs or
answers cannot land in a newly opened World with coincident coordinates.

## Character, movement and camera

One Server-owned Player supports noclip free flight and physics movement
with gravity, Shape-aware collision, walking, sprinting and jumping. Use
core collision in metres; a Client pose speed limit is not physical authority.
Do not give Client systems World storage access for movement or camera rays.

New demo Worlds start physics/first-person; a no-Game empty World retains
the existing free-flight fallback. Reopen restores saved Player
position, movement mode and view. Free flight is an explicit mode switch;
refuse switching to physics while embedded in solid terrain, with feedback
rather than an unexpected teleport. Mode permissions are Server/Game policy.

Game supplies a simple blocky humanoid with idle/move/sprint/jump animations;
Engine owns presentation mechanisms. Full body is visible in third-person.
First-person hides it and retains the existing held model. Character creation,
skins/cosmetic inventory and a new first-person arms system are deferred.

First-person and rear third-person work independently of movement mode.
Provide remappable view switching. Third-person is a centred rear chase
camera with zoom and wall avoidance, not front/selfie or cinematic views.
Crosshair selects the camera-visible point; Server validates reach and
occlusion from the Player's Eye. Invalid/blocked points produce feedback,
not a surprising fallback Edit. Camera offset cannot move the Player or grant
extra reach. Normal wheel selects quick slots; remappable Zoom In/Out actions
use Alt+wheel in third-person by default. No accidental control stealing.

## Architectural invariants

- Engine ships only named Material Air; retired ids remain retired and Mod
  ids start at 8. Game owns ground, artwork, character/tool content and policy.
- Server owns World changes, aim answers, Player authority and persistence.
  Client renders content/answers and sends intents; no World storage reads.
- Mods use bounded batched WIT, never direct core calls. Contract/version and
  adapter changes require review of actual release state before implementation.
- Core remains independent of Bevy/wgpu. All physical quantities use metres.
- Preserve exact Occupants, Lifetime, edit/light/mesh/LOD invalidation and
  history guards. Simulation consequences elsewhere are not rewound by undo.
- Respect ADR-0008/0011 boundaries and amend their deferred movement/save
  decisions in the corresponding PRs, not by silently violating them.

## PR boundaries and ticket map

Each PR stays narrow. Dependencies do not authorize bundling unrelated work.
Start with P0 baseline/profiling, tickets 11–12 only in the current branch.
Implement inline and sequentially, one PR at a time, with a separate branch/
worktree for each PR. Later scopes start after their prerequisites. Continue
in the same session where practical; no parallel coding agents or extra
threads are planned. Touch is not first by default.

| Tickets | PR | Depends on |
| --- | --- | --- |
| 11–12 | P0: frame profiling and release fixtures | none |
| 01–02 | C0: ownership and semantic actions | none |
| 03–04 | C1: controls profile, settings and behavior | C0 |
| 28 | P5: display sync, caps and uncapped settings | C1 profile/UI |
| 05–06 | C2: focus, search and readable catalogue | C0; C1 labels |
| 13 | P1: static atlas optimization | P0 evidence |
| 14 | P2: quiet UI optimization | P0 evidence |
| 15 | P3: one per-view render optimization | valid GPU/per-view evidence |
| 16 | P4: emitter relight optimization | simulation proof/evidence |
| 17 | I0: creative organization and shortcuts | C0, C2, profile storage |
| 22 | S0: World save/lifecycle schema | agreed save policy |
| 23 | S1: durable World/simulation changes | S0 |
| 27 | W0: World list/create/seed UI | C2, S0, S1 |
| 24 | M0: Server-owned flight/physics movement | C0 |
| 25 | A0: Game character presentation | M0 |
| 26 | V0: first/third-person view and aim | M0, A0 |
| 20 | T0: explicit Use/Interact contract | C0 |
| 21 | T1: measurement tool | T0, C2 |
| 07 | C3: controller | C0, C2, controls settings |
| 08–10 | C4: desktop touch and hardware acceptance | C0, C2 |
| 18–19 | I1/I2: owned inventory and resource saves | deferred |

P3/P4 remain evidence-gated; they do not imply an approved speculative
algorithm. I1/I2 remain deferred, not implementation-ready. Real controller/
touch tests and user visual review are acceptance requirements, not claims
this Mac's injected input tests can satisfy.

## Acceptance, build and review

- Preserve current defaults and Server guards while testing all new modes.
  Replay intent traces at 60/144/1000 render updates; allow one render interval
  of timing quantization, never duplicate actions or transferred hold progress.
- Test focus/menu/device changes, capture clicks, simultaneous inputs,
  cancellation, settings Apply/Cancel/conflicts/reset and corrupt profiles.
- Exercise complete keyboard, controller and touch catalogue journeys,
  long/missing/duplicate names, empty Game content and the UI size matrix.
- Test save/load/evict/reopen, dirty-save races, partial writes/recovery,
  missing content, same-seed separate Worlds and random/edge seed inputs.
  Verify no startup construction overwrites a reopened edited demo World.
- Test paused simulation resume, Player restore, movement against shaped/
  unloaded terrain, invalid mode switches and third-person aim/obstruction.
- Test measurement corner selection, range, read-only actions and session
  replacement/clear. No owned inventory or resource transaction is implied.
- Preserve phase-8.2 ticket 13's thread-CPU A/B methodology: unique targets,
  executable hashes, foreground QoS, interleaving, equal output and no competing
  build/benchmark. Character/save/camera fixtures record movement/view mode.
- Cargo uses caffeinate -i and RUSTFLAGS="-D warnings", shared Bevy cache and
  --workspace --features tesserae/dev where applicable. Rebuild guests/demo
  after WIT/Game changes. Run relevant regressions and required CI checks.
- Format Rust/tool-owned docs appropriately; never run Prettier on .scratch.
  Wrap this Markdown at 80 columns. No native PNG/trace/binary in git.
- Put native review captures in one unique /tmp folder, use offscreen capture
  for a locked display and retain until user review. Physical hardware tests
  and visual review are recorded separately. No unsupported support claims.

## Confirmation state

All user-facing questions Q1–Q49 are accepted. Remaining technical choices
are implementation/design work under these constraints, not invented user
requirements. Hardware access and measured optimization evidence remain
validation gates. The user confirmed shared understanding after the full
interview; local implementation is authorized. Push/publication/merge remain
separate actions without authorization.
