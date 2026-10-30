# Desktop gameplay, World persistence and measured performance

Status: shared understanding confirmed; P0 merged; C0 is next planned scope.

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
ok after the consolidated plan. P0 was subsequently implemented and its
publication authorized; PR 22 is now merged. Later scopes do not start
from this cleanup, and publication/merge permissions do not carry forward
automatically. PR 21 remains closed; its approved phase-8.2 spec stays intact.

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

Preserve current defaults: hold removal, right place/ignite, X empty hand,
middle/P sampling and fire > solid > source removal priority. Sprint toggles
by default (user decision 2026-10-29, ticket 27); a saved Hold choice is
kept, and Hold stays a preference.
Keep Server aim/history guards and one removal per activation, except under
the opt-in "Repeat while held" preference of ticket 29.

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

The user accepted the existing main catalogue/menu behavior as complete for
tickets 05 and 06 after reverting the expanded PR 3 UI. Their narrowed ticket
scopes supersede this section's earlier additional requirements. No further
catalogue redesign or expanded search/sizing work is implied by these tickets.

Keep mouse quick-hold, visible Assign to slot, drag/drop and number shortcuts.
Selecting content keeps Catalogue open; choosing a Shape updates the held
variation. Clicking the held content again or Clear selection empties the hand.
An explicit Material selection pins its Shape panel while other entries may
still highlight on hover. Keep drag sources alive until completion.

Search declared Material names with the existing editor and empty-result
feedback. Retain search text across menu pages, and keep typing isolated from
gameplay input. Use the existing short tile labels, full Material identity in
details and named Shape choices. Preserve wrapping cards, the fixed quick-slot
footer and its horizontal scrolling. Catalogue/settings leave Server simulation
running. Pausing is separate work. Controller and desktop-touch journeys remain
in their separately planned scopes below.

## Desktop controller and touch

Controller sticks directly move/look during play; menus use focus navigation,
not a virtual mouse cursor. Prompts follow deliberate last-used input, not
drift. Controller bindings, sensitivity, invert Y and deadzones are separately
configurable without replacing keyboard/mouse preferences. Apply trigger
hysteresis and cancellation on disconnect. Validate default mappings on real
hardware before claiming support; injected tests are not physical validation.
Controller grilling (2026-10-29) is recorded in ticket 07, now done and
validated on Xbox, PlayStation and Switch/8BitDo pads. The Server reads the
movement direction's length, capped at 1, as a throttle (ADR-0008), so
analog input scales speed and keyboard speeds stay the same. Rumble,
Nintendo glyphs, an on-screen keyboard and split-screen are excluded. Pad
defaults follow Minecraft Bedrock: B toggles crouch and LS click latches
sprint. Optional repeat place/remove while held is device-neutral ticket 29.

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

Touch grilling (2026-10-30) is recorded in tickets 08–10. Touch is delivered
only on Windows and Linux (winit 0.30 emits no touch events on macOS), so
macOS is unclaimed and tests inject `TouchInput`. Touch has fixed, explicit
controls and is not remappable; there is no tap-the-world action. Contacts keep
one role from start to end/cancel and are cancelled by focus loss, menus and
device switches. Synthetic mouse events are suppressed in our reader, with a
grace period tuned on hardware. 08 and 09 are one PR; 10 is a human-run record
and touch stays "implemented, not validated" until it passes.

## Creative organization and interaction

The user accepted the existing creative shortcut workflow as complete for
ticket 17. Its narrowed scope supersedes earlier additional organization-UI
requirements. Keep nine shortcuts plus temporary outside selection, with no
bag grid or extra pages. X and Clear selection empty only the hand. Existing
assignment and drag/drop replace or swap shortcuts; an invalid outside drop
cancels rather than deleting a shortcut. No further slot-operation UI is
required by this completed ticket.

Save arrangements per Game using stable Material/Shape names and Orientation.
Every World opens empty-handed; the saved shortcuts and selected slot are
kept, and a slot key or click picks one up (user decision 2026-10-29).
Restore across restarts; missing content is visibly empty, not a different
item at a reused id. Controls remain global per user. Temporary outside-held
selection is not persisted. These shortcuts are not owned resource saves.

Use/Interact is a dedicated remappable action. Right click stays place/ignite;
there is no use-before-place or automatic fallback. Empty hand, sampling,
pickup and use remain distinct. Finite stacks, consumption, physical pickup,
durability and survival undo are later policies, not approved by this spec.

Use is a separate Server intent with its own outcome and refusals; it never
journals and never repeats while held. A tool is a hand state: holding one makes
place, remove and undo/redo inert. With an empty hand or any non-tool hand Use
is a quiet no-op. A Game declares tools once in the unreleased 0.3 declaration
as data; the Server runs Use natively, with no host call (grilled 2026-10-30,
tickets 20-21). Default keys: CycleTool `M`, which takes the next declared tool
or puts it away (never listed in the Catalogue); Use `G` / pad `DPadUp`;
ClearTool `Backspace`.

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

Measure aims at solids only, through flowing fluid and fire. The Server stays
stateless: the Client holds up to two endpoints, cleared by ClearTool or a
World switch and never persisted. Metres are the source; Voxel units are
derived. Endpoint markers and line show in both views, with a corner readout
while the tool is held.

## World creation, seed and persistence

Provide a World list with Create, Open and Rename. New World asks for name,
installed Game and optional seed; with no Game installed, Create is disabled
(user decision 2026-10-29, ticket 27: an empty World was only Air). Mod download/management
is separate; keep explicit Trust checks before running Mod code.

Blank seed chooses a random seed; Randomize can choose another. Accept
numbers and words, with stable documented text conversion. Display/copy the
resolved numeric seed and save it with World settings. Matching seed,
Game/content, generator version, parameters and Voxel size reproduce starting
terrain; distinct seeds need not produce distinct terrain. Changing an
existing World's seed in place is not supported. Different Worlds may share
a name/seed without sharing save identity or overwriting each other.

Ticket 27's 2026-10-29 grilling fixed the details: the list re-execs the
process per World (in-process sessions are ticket 30), folders get readable
names with a sidecar beside unchanged commits, Delete uses the OS Trash,
Create offers the tested Voxel sizes under More World options, and the World
screens are mouse/keyboard only until tickets 32 (controller) and 08–10
(touch) add their journeys. Multiple installed Games are ticket 31.

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
Do not give Client systems World storage access for movement or aim rays.
Camera collision is presentation: the Client sweeps against the Chunk meshes
it was sent, never World storage.

New demo Worlds start physics; a no-Game empty World retains the existing
free-flight fallback. Reopen restores saved Player position, movement mode
and orientation. View mode and zoom are a per-user Client preference in the
user profile, not World save data; first launch starts first-person.
Free flight is an explicit mode switch;
refuse switching to physics while embedded in solid terrain, with feedback
rather than an unexpected teleport. Mode permissions are Server/Game policy.

Game supplies a simple blocky humanoid with idle/move/sprint/jump animations;
Engine owns presentation mechanisms and an untextured fallback body for
Worlds whose Game declares none. Full body is visible in third-person.
First-person hides it and retains the existing held model. Character creation,
skins/cosmetic inventory and a new first-person arms system are deferred.

First-person and rear third-person work independently of movement mode.
Provide remappable view switching. Third-person is a rear over-the-shoulder
chase camera with zoom and wall avoidance, not front/selfie or cinematic
views. A camera centred on the look line would put the avatar's head over
the crosshair target.
Crosshair selects the camera-visible point; Server validates reach and
occlusion from the Player's Eye. Invalid/blocked points produce feedback,
not a surprising fallback Edit. Camera offset cannot move the Player or grant
extra reach. Normal wheel selects quick slots; remappable Zoom In/Out actions
use Alt+wheel in third-person by default. No accidental control stealing.

The 2026-10-29 character/camera grilling, and its review against Minecraft,
Luanti, Valheim and Enshrouded, settled the detail in tickets 25 and 26. The
Server does not know the view mode. Aim starts from the camera ray but is
validated from the Eye.

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
P0 baseline/profiling (tickets 11–12) is complete; C0 (01–02) is next.
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
| 17 | I0: creative organization and shortcuts | C0, C2, profile storage |
| 22 | S0: World save/lifecycle schema | agreed save policy |
| 23 | S1: durable World/simulation changes | S0 |
| 27 | W0: World list/create/seed UI | C2, S0, S1 |
| 30 | W1: in-process World sessions (W1a-c) | W0; ready |
| 31 | G0: multiple Games, per-World Mods | W0; needs triage |
| 32 | W0.1: window, keyboard and controller follow-ups | W0, C3; needs triage |
| 33 | water immersion: view (A), movement (B) | M0, V0; ready |
| 24 | M0: Server-owned flight/physics movement | C0 |
| 25 | A0: Game character presentation | M0 |
| 26 | V0: first/third-person view and aim | M0, A0 |
| 20 | T0: explicit Use/Interact contract | C0 |
| 21 | T1: measurement tool | T0, C2 |
| 07 | C3: controller | C0, C2, controls settings |
| 08–10 | C4: desktop touch and hardware acceptance | C0, C2 |
| 18–19 | I1/I2: owned inventory and resource saves | deferred |

P0 merged as [PR 22](https://github.com/mkappworks-dev/tesserae/pull/22).
C0 (tickets 01–02, sequence position PR 2) is the next planned scope.

Conditional performance experiments are outside the implementation sequence:

- Ticket 13 / P1: attribute recurring atlas cost and validate an on-demand
  candidate at unchanged appearance.
- Ticket 14 / P2: attribute idle UI/system/layout cost before reducing work.
- Ticket 15 / P3: obtain usable per-view CPU/GPU evidence, then select one
  bounded candidate.
- Ticket 16 / P4: attribute emitter relight in targeted simulation fixtures
  and prove bounded equivalent outcomes before code.

P0 supplied instrumentation and a qualified baseline, not measured gains
from these four candidates. None is implementation-ready or blocks C0.
Promote one to an implementation PR only after its ticket-specific evidence
and stable matched comparison justify the complexity. A failed experiment
may remain deferred or be closed without implementation. This clarifies
scheduling; all accepted quality, ownership and simulation rules remain.

P1–P4 remain evidence-gated; no speculative algorithm is pre-approved.
I1/I2 remain deferred, not implementation-ready. Real controller/touch
tests and user visual review are acceptance requirements, not claims
this Mac's injected input tests can satisfy.

## Acceptance, build and review

- Preserve current defaults and Server guards while testing all new modes.
  Replay intent traces at 60/144/1000 render updates; allow one render interval
  of timing quantization, never duplicate actions or transferred hold progress.
- Test focus/menu/device changes, capture clicks, simultaneous inputs,
  cancellation, settings Apply/Cancel/conflicts/reset and corrupt profiles.
- Exercise the accepted catalogue selection/assignment, Material search and
  retained main layout. Controller/touch journeys belong to their later
  tickets; they do not reopen the completed catalogue scope.
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
interview. P0 merged under subsequent user authorization. Later work and
its publication/merge remain separate actions requiring applicable user
instructions; this planning cleanup starts no implementation.
