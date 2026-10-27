# Separate PR proposals

Status: historical research proposals, superseded by spec.md.

The completed Q1–Q49 interview and reconciled spec.md are authoritative.
This file preserves the earlier exploration; alternatives, priorities and
readiness statements below are not new open questions or implementation
instructions. See spec.md, grilling.md and issues/ for current scope.

## Ordering and independence

Performance and input work can proceed independently after the merged phase
8.2 baseline. Keep every scope on a separate feature/tech branch and PR.
Labels here identify proposals, not ticket numbers or a new roadmap phase.

| Scope | Dependency | Priority |
| --- | --- | --- |
| P0: reproducible frame profiling | none | first performance PR |
| P1: render static item atlas on demand | P0 evidence | first small candidate |
| P2: stop idle UI invalidation | P0 evidence | small candidate |
| P3: reduce view/render work | P0, render evidence | investigate |
| P4: batch emitter relight safely | simulation measurements | later |
| C0: actions and input ownership | none | first controls PR |
| C1: editable keyboard/mouse bindings | C0 | next |
| C2: focus and touch-safe catalogue | C0; C1 labels later | next |
| C3: controller input and navigation | C0, C2 | separate device PR |
| C4: desktop touch interaction | C0, C2; hardware | device PR |
| I0: explicit creative selection rules | none | before inventory expansion |
| I1: item transactions | I0; item policy | domain PR |
| I2: inventory persistence | I1; World/Registry saves | later |
| T0: use/interaction contract | C0; I1 if consumed | before tools |
| T1: one Game-declared tool | T0; inventory if consumed | one tool per PR |

P1 and P2 require their own A/B evidence; neither is permission to change the
approved appearance. P3 is not a renderer rewrite. P4 must preserve ADR-0014.
Controller does not depend on touch, and touch does not depend on controller.
Creative selection does not have to wait for survival or network transport.

## C0: semantic actions and a single input owner

Journey: open the catalogue while moving, browse or assign an item, then
return to play. The same physical input must have one owner throughout.

Current input is spread across `camera`, `edit`, `menu`, `history`, `help`,
`pick` and `removal`. Edit/history gestures gate on mouse cursor locking;
camera flight and respawn do not gate on the menu. `select_held` has ad hoc
menu handling. Ctrl+Z suppresses descent with a special case in `camera`.
This is a modest foundation change, not a reason to rework phase 8.2 now.

Scope:

- Define stable action ids for movement axes, look, sprint, speed, respawn,
  remove, place/ignite, sample, empty hand, slots/slot step, turn, flip,
  Shape cycle, catalogue, help, diagnostics, undo, redo and cancel.
- Resolve device input once into action state: started, held, released,
  cancelled, axis value. Preserve deliberate hold and one action per hold.
- Explicit contexts: gameplay, catalogue, binding capture, text entry and
  unfocused. UI owns events it handles. Cursor capture is an outcome of
  context, rather than the definition of gameplay for all devices.
- Specific modifier chords consume their triggers before simpler bindings;
  overlapping actions are either intentionally compatible or diagnosed.
- Context change, focus loss, device disconnect and menu/cursor changes
  cancel ongoing gestures. Returning to play requires a fresh activation.
- Preserve Server intent/message boundaries and existing mouse defaults.

The Server keeps simulating while UI owns input; pausing needs a separate
Game policy. Device input support is separate from the physical Player
controller (gravity, collision and prediction) deferred in ADR-0008.

Acceptance: existing hold, sampling and history tests still pass; the same
recorded action trace produces the same intents at 60, 144 and 1000 render
updates/s; no click-through on capture, close or assignment; Ctrl/Cmd+Z
cannot descend or initiate a tool; typing in a menu does not move or respawn.
No direct World read is introduced. Include an explicit simultaneous-button
and focus-loss trace, not tests that merely mirror an enum.

## C1: keyboard/mouse remapping and settings

Journey: a left-handed player remaps movement, assigns sample to a mouse
side button, resolves a conflict, restarts and sees matching help labels.

Scope:

- One settings view with primary/secondary bindings, unbind, per-action and
  full reset. Escape cancels capture; modifiers alone wait for a trigger.
  Capture starts after the initiating click is released.
- Support keys, mouse buttons and wheel directions. Movement axes keep
  opposing directions; raw mouse motion remains a separate look axis.
- Detect conflicts only in overlapping contexts. Offer replace or cancel;
  intentionally shared modifiers such as sprint are not blanket conflicts.
  Keep a documented emergency cancel path so the player cannot trap themself.
- Defaults exactly preserve X, middle/P, Q/E/F, R, Tab, I, F3, undo/redo and
  the approved left-hold/right-place semantics. Reserved OS shortcuts cannot
  be guaranteed to reach the app; explain an unavailable binding at capture.
- Persist a versioned Engine user profile outside the World and Game. Atomic
  replacement, readable corrupt-file recovery and default merging for new
  action ids. Remapping must not rewrite a World or a Mod Manifest.
- Bind physical movement positions by default, retain stable serialized
  ids, and derive readable layout-aware key labels. Use platform-specific
  Ctrl/Command labels, with a fallback for unknown layouts/devices.
- All help, catalogue instructions and placement/removal cues derive from
  current bindings. Do not leave literal key strings in product copy.

Acceptance: remap, restart, reset and corrupt-profile recovery work; AZERTY
or another non-US layout is exercised; both modifier sides and all three
redo defaults resolve correctly; rebind capture sends zero World intents.
Full localization is a separate scope; action ids and label construction
must leave room for it. The current embedded ASCII font needs a font asset
with coverage before claiming localized names or shortcut glyphs.

## C2: focus, accessibility and catalogue navigation

Journey: a keyboard-only player finds a named Material, chooses its stair
Shape and assigns slot 4 without a pointer. A player with reduced vision
increases UI size without losing the fixed slot footer.

Scope:

- Visible focus, predictable Tab/arrow navigation, activate and cancel.
  Restore focus to the invoking control after closing settings or a dialog.
  Focus must select the Shape panel just as hover does today.
- Catalogue search over declared names and Shapes; simple content-kind or
  namespace filters. Game-defined categories need a separate content contract
  decision, so do not invent metadata in the Client.
- A select-then-assign path alongside drag/drop and hover-number shortcuts.
  Keep browse position and focus stable as the Shape panel changes.
- Expose UI scale, readable label wrapping and font size. Define a minimum
  supported logical viewport and test footer overflow explicitly. Do not
  assume more physical Retina pixels mean more layout space.
- Text/icons as well as colour for selection, validity and accepting slots;
  sufficient contrast and a clearly visible focused tile. Reduced motion,
  invert look and sensitivity are appropriate settings; hold accessibility
  can offer a cancellable latch while preserving one action per activation.
- Discoverable help and binding labels. Keep help and diagnostics separate.
  Treat screen-reader announcements as a separately verified capability;
  merely adding a focus outline is not screen-reader support.

Acceptance: full keyboard journey including slot swap and cancel; no
unreachable control at the agreed minimum viewport and 100/150/200% scales;
long namespace names and empty search results remain readable; colour alone
never conveys a refusal; a search keystroke never reaches gameplay.

## C3: controller

Journey: connect a controller during play, move/look, deliberately remove a
Voxel, open the catalogue, assign a slot and continue without touching a mouse.

Scope: analog movement/look with independent sensitivity, radial deadzones,
response curves and invert Y; trigger thresholds with hysteresis; digital
actions through C0; focus navigation through C2. Propose left trigger remove,
right trigger place, shoulders slot step and a face button for catalogue,
then validate ergonomics on hardware. Keep the action set independent of one
vendor's glyphs. Show prompts for the last deliberately used device, with
hysteresis so stick noise does not replace keyboard labels. Disconnect cancels
holds and releases UI capture. Never key gamepad actions to a locked mouse.

Acceptance: one tested controller per supported OS before claiming support;
connect/disconnect during hold, focus loss and menu cancellation send no
extra Edit; drift causes no movement; look is stable at different FPS;
every catalogue/settings action has a controller path and readable prompts.
Haptics can be a later optional enhancement after success is Server-confirmed.

## C4: touch, separately from controller

Scope clarification: the whole plan is desktop-only. This touch proposal
does not reopen mobile/web support or establish touch as the next feature.

Journey: on a supported desktop touchscreen, one thumb moves while another
looks; dedicated place/remove controls act on the shown target. Opening the
catalogue cancels the gesture; scrolling it never mines terrain underneath.

Scope: stable pointer-id ownership, multi-touch cancellation and explicit
move/look zones; separate tap-place and hold-remove controls; slot selection
and select-then-assign without hover. Proposed targets are at least 44 logical
pixels with spacing and adjustable HUD placement. Avoid a long press that
ambiguously means both inspect and remove. Preserve target/hold cancellation
and Server target authority. Synthetic mouse events from touch must not cause
duplicate actions. A mouse remains usable after touch input.

Acceptance: real touchscreen two-finger movement/aim and place/remove tests;
interrupted touch, window focus loss and menu transitions cause no edits;
no dependency on hover, middle click or keyboard numbers; usable UI at the
agreed viewport/scale. Native device tests are required before marking this
complete; this Mac setup alone cannot establish touch usability.

Desktop touchscreen input preserves the current platform policy.
iOS/Android/web support remains outside this effort under `DESIGN.md`'s
explicit non-goals. Any future expansion needs a separate platform decision,
build/runtime work and performance budget.

## I0: creative selection rules

Journey: choose a stair temporarily, put water in slot 8, empty the hand,
sample a placed stair and return to the original slots without losing them.

Document and test the existing mode as unlimited creative selection: the
catalogue is content availability, the hotbar is shortcuts, and the outside
item is a temporary selection. X changes selection, not ownership. Sampling
is a read-only Server answer preserving Shape and Orientation; it neither
removes a Voxel nor grants an owned item. Removal is not physical pickup.
Keep this PR separate from stacks, crafting, drops and survival. Saving
creative shortcuts may be a versioned user preference by stable content name,
not runtime Material id; missing content falls back visibly to an empty slot.

Acceptance: all current creative journeys remain intact, including nine
unchanged slots across X/sample/outside selection. Content ids can change
between loads without selecting a different Material by accident.

## I1: Server-owned inventory transactions

Journey: place the last item in a stack, receive a Server acknowledgment,
then attempt a second placement; it refuses without a negative count or a
ghost that still promises success. Two pickup requests cannot duplicate a drop.

First define an Item independently of a Voxel: a material/Shape item, tool,
container and future recipe ingredient are not all the same type. Game
declarations own item definitions, maximum stack sizes, drops, allowed modes,
consumption and durability rules. Engine owns transaction mechanics.

Scope: per-Player Server inventory, stable slot identities and revisions,
idempotent operation ids, exact accepted quantity and refusal reasons. Place
and consume commit atomically with the Edit; pickup removes a world entity
and grants capacity atomically. Client sends move/split/use/pickup intents
and displays snapshots/deltas, never its own authoritative count. Validate
permissions, stack identity and capacity on the Server. Transport can remain
in-process, but messages must identify the Player and tolerate retries.

Acceptance: duplicate, stale and reordered requests cannot duplicate/loss
items; full inventory and rejected placement preserve both sides; per-Player
isolation and state hashes cover transactions. Creative mode remains unlimited.
Physical pickup, drops, crafting and death loss should each be later scopes.

Undo is a required policy decision before enabling survival: today's journal
stores Occupants, not inventory receipts. Recommended first release disables
creative World undo in survival; transaction-aware undo, if desired later,
must atomically reverse resource effects and reject already-spent receipts.
Never refund an item based only on restored World state.

## I2: persistence and network implications

No durable World save exists today. Persisting only inventory would allow
placed Voxels and owned items to disagree after reopening. Depend on durable
World/Registry identity and a crash-consistent transaction/save boundary.
Store stable item names and schema versions; specify missing-Mod handling,
tool/container state and recovery. Save compatibility and migration need
their own reviewed design. Do not serialize runtime Material ids or Bevy
entities as durable ownership.

Acceptance: restart and interrupted-save recovery preserve total ownership;
reconnect/replay cannot apply a transaction twice; unknown content has an
explicit recoverable policy. This scope does not smuggle network transport
into the input-remapping PR.

## T0 and T1: actions before a collection of tools

Journey: approach something interactable with empty hand, see what use will
do, then use it and receive a success/refusal. A mining tool still removes
only the Server-confirmed target after its declared time.

T0 defines an explicit use/interaction intent and Server capability answer,
including target identity, action availability and refusal. Keep empty hand,
sampling, physical pickup and use distinct. Preserve current left removal,
right place/ignite and fire > solid > source removal priority until a new
mapping is explicitly approved. An explicit separate Use action is the lowest
ambiguity starting option. Right-click use-before-place is familiar but needs
a deliberate bypass-to-place rule and corresponding ghost/cue; it is an
alternative design decision, not an automatic implementation change.

Do not put tool-specific branches in the Engine for named Game items. Add
batched WIT capability/declaration/result data where necessary, through the
versioning/adapter policy. Review release state before choosing whether to
amend an unreleased line or introduce a new one.

T1 should implement one end-to-end tool, not a toolbox:

- Mining tool: Game-declared effective Materials and timing, Server authority
  over eligibility/resource changes. Current Client hold timing is a gesture
  guard, not anti-cheat; survival timing requires Server elapsed-time checks.
- Container tool: collect a source into capacity and place it atomically,
  preserving contained fluid identity. Decide separately whether flowing
  fluid is collectible; never silently drain adjacent sources or stored solid.
- Measurement tool: Server-returned metre endpoints/distances, read-only;
  independent of survival inventory and a useful lower-risk first tool.

Choose one per PR with a successful action, refusal, cancel, stale-target,
mode and resource test matrix. Mining/building cues should say what is
targeted, whether it is valid, progress and Server-confirmed result. Optional
sound/haptics follow confirmation. No hand swing is essential to correctness.
Bulk selection/fill tools depend on bounded bulk edit/relight work and their
own transaction design; do not multiply the existing single-Edit path.

## Finish criteria for the existing work

PR 21 is already merged with successful checks; no work in these proposals
requires reopening it. No new evidenced correctness defect has been
established by this investigation. Existing tests cover capture-click
suppression, menu cancellation, guarded removal, stale aim and history guards.
The research driver does not establish physical mouse/controller/touch input
correctness or multiplayer safety.

Stop/release-blocking evidence would be a reproduced menu/capture click
editing the World, a Client storage access, duplicate Edit/item transactions,
an unload/undo guard failure, a crash or a shader/content regression. None
should be inferred from a frame counter or from planned device support.

Actual follow-ups, in order: explicit input ownership and focus; rebindable
actions with correct labels; readable keyboard-navigable catalogue; controller
and touch independently; inventory/domain rules before consumable tools.
Search, categories, sound/haptics and animation are later conveniences, not
reasons to extend approved phase 8.2.

## Decisions worth asking after reviewing these options

Desktop-only is the platform boundary, not a priority decision. Grill the
next desktop feature and its intended behavior before selecting work.
Additional unresolved decisions include:

1. Keep the next gameplay iteration creative, or prioritize survival ownership
   and durable World saving? Recommended: creative controls first.
2. First tool: read-only measurement, mining effectiveness, or a source
   container? Recommended: measurement if the goal is testing the action API;
   mining if survival is the next approved direction.
3. Keep a separate Use binding initially, or design use-before-place with a
   bypass? Recommended: explicit Use until target capabilities are proven.

No answer is needed to preserve the current behavior. These decisions gate
future implementation approval, not this research work.
