# 07: C3: Controller input, settings and validation

Status: ready-for-human
Blocked by: 02, 04, 06

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: C3; one PR on `feat/controller`, titled
`feat(controls): controller input, navigation and settings`.

## What to build

Adapt controller input to semantic actions: direct analog move/look in play,
fixed focus navigation in menus, no virtual mouse. Decisions from the
2026-10-29 grilling (Q1–Q28) are binding. Defaults follow Minecraft Bedrock's
controller layout where Tesserae has an equivalent action.

### Input source and plumbing

- Use Bevy 0.20's built-in `Gamepad` entities, `GamepadButton` and
  `GamepadAxis`. `bevy_gilrs` is already enabled by Bevy's default features;
  add no crate and make no direct gilrs/SDL calls.
- Put the pad adapter in a new `client/pad.rs` that writes into the shared
  `Actions` frame. `camera.rs` consumes axes only and never knows the device.
- Add `Axes.move_: Vec2` and `Axes.look_rate: Vec2` (per second) beside the
  existing mouse `look` delta. Add a device-neutral `Device { KeyboardMouse,
  Pad }` enum that desktop touch (tickets 08–10) extends with its own adapter.

### Bindings and profile

- Add `Trigger::Pad(PadButton)` and a separate `pad` binding set in the same
  profile document: two alternatives per action, no modifiers. Load it with
  `#[serde(default)]`, so older profiles without the section stay valid with
  no version break.
- Keyboard/mouse bindings, preferences and conflict checks are untouched.
  Check pad conflicts only against pad bindings in overlapping contexts.
- Add pad preferences separate from the mouse fields: `pad_look_sensitivity`,
  `pad_invert_y`, `pad_move_deadzone` (default 0.15), `pad_look_deadzone`
  (default 0.15) and `pad_toggle_crouch` (default on).

### Default gameplay mapping (Xbox naming, remappable)

| Input | Action |
| --- | --- |
| Left stick / LS click | move (analog) / Sprint (latch) |
| Right stick / RS click | look (analog) / Sample (pick aimed target) |
| South (A) | Ascend / jump |
| East (B) | Descend / crouch (see below) |
| North (Y) | Catalogue (opens and closes) |
| West (X) | Shape |
| RT / LT | Remove (hold) / Place, ignite |
| LB / RB | previous / next quick slot |
| D-pad left / right | Turn / Flip |
| D-pad down | EmptyHand |
| D-pad up | unbound; reserved for view switch |
| Start / View | Help / MovementMode |
| Guide / Home | never bound (macOS game overlay) |

Undo, Redo, Respawn, Faster, Slower, Diagnostics, ReleaseCursor and Slot1–9
have no pad default but are bindable, except ReleaseCursor, which has no pad
meaning. View switch from the cameras scope takes D-pad up; other new actions
get pad defaults in a follow-up rather than guessed here.

### Crouch and sprint on the pad

- With `pad_toggle_crouch` on, pressing B in physics movement toggles crouch.
  The state lives in a Client resource like toggle sprint, and the existing
  `crouch` intent field carries it. It clears on menu, focus loss, respawn,
  mode switch and jump. In flight, holding B descends. Keyboard crouch stays
  hold. Reason: the right thumb cannot hold B and steer look at once.
- LS click latches sprint on until the move stick returns inside its deadzone
  or crouch engages. It also clears on menu, focus loss and respawn. This is
  independent of the keyboard `toggle_sprint` preference, because holding a
  stick click while pushing that stick is not physically workable.

### Sticks, triggers and hold

- Both sticks use a scaled radial deadzone. Look uses a mild exponent curve
  integrated over `dt` in degrees per second, so it is FPS-independent. Do
  not add swap-sticks or per-axis options.
- Trigger hysteresis uses Bevy's `GamepadSettings`/`ButtonSettings`: press at
  0.6, release at 0.4. It is not user-configurable in this PR.
- RT feeds the existing `Remove` action unchanged. Hold timing, tap-remove,
  one removal per activation and the Server guards all apply as they do for
  the mouse.

### Server movement contract

- In both the fly and physics branches of `server/player.rs`, replace
  `direction.normalize_or_zero()` with `clamp_length_max(1.0)`. A half-pushed
  stick walks at half speed. Digital input is unchanged: one key gives 1, and
  diagonals clamp from √2 to 1.
- The Server still caps speed (ADR-0008). There is no WIT, core or
  `MovementIntent` field change.

### Menus, focus and text

- In UI contexts (Catalogue, Settings, Modal), navigation is fixed and not
  remappable: d-pad or left stick moves focus via `AutoDirectionalNavigation`
  (with repeat on hold), South activates, East is Back through the existing
  `consume_cancel` route, LB/RB switch tabs, and Start closes. These are
  reserved like Escape, so a user cannot lock themselves out. Y also closes
  the Catalogue it opened.
- When a menu opens with Pad as the last device, focus starts on the held
  item, or on the first tile if the hand is empty. The focused element is
  always scrolled into view. On a focused slider, d-pad left/right steps the
  value instead of moving focus.
- Directional navigation skips text fields. There is no on-screen keyboard,
  and every accepted journey must be completable without typing.
- Catalogue assignment works without drag: focus a tile, South selects it into
  the hand, pick a Shape if needed, then use the existing Assign to slot
  control and pick a slot. Drag/drop stays mouse-only. If a control is not
  focusable, make it focusable; do not redesign the catalogue (tickets 05,
  06 and 17 stay closed).

### Settings and capture

- Add a Controller category to Settings with the pad bindings and pad
  preferences. It reuses the draft/Apply/Cancel, conflict replace/cancel and
  per-section reset flows unchanged.
- Capture starts after the opening South is released. A pad row accepts only
  pad buttons, and a keyboard/mouse row only keyboard/mouse. Start and East
  cancel a pad capture and are reserved during capture.

### Ownership, devices and prompts

- Pad input goes through `Ownership`. A pad button held across a context
  change is ignored until released.
- Ignore pad input entirely while the window is unfocused, because gilrs
  reports input even then.
- In `PointerFree`, a pad South press re-grabs the cursor and returns to
  Gameplay. That press is consumed and is not a jump. In menus with Pad as the
  last device, hide the cursor until the mouse moves.
- Exactly one active pad: the most recent to make a deliberate input. All
  connected pads are read.
- Connect/disconnect of the active pad bumps the device epoch, cancelling
  holds and axes. Another pad takes over only after a fresh deliberate press.
  No split-screen or per-player pads.
- Prompts follow the last deliberate device:
  - Pad: a pad button press, or a stick/trigger exceeding 0.5 past its
    deadzone.
  - Keyboard/mouse: any key, mouse button, wheel, or mouse motion over a small
    threshold.
- Drift never switches device. With Pad active, help, cues and the play bar
  show pad labels via `Bindings::label` as text.
- Use two label sets, chosen by vendor id:
  - Sony (`0x054C`): PlayStation names (Cross, Circle, Square, Triangle, L2,
    R2).
  - Every other pad: Xbox names ("A", "RT", "D-pad ↑").

### Placement

- Everything above is Engine Client: per-user input on one machine, stored in
  the global profile. The Server change is only the magnitude clamp.
- Mods see semantic actions only, across WIT (ADR-0001). Games and Mods cannot
  declare pad defaults in this PR. Once Game-declared actions exist (after
  ticket 20), they may suggest per-device defaults through a declaration, never
  by reading raw input.

### Out of scope

- Rumble: gilrs-core 0.6.8 on macOS reports `is_ff_supported() == false`.
- Nintendo glyph sets, and controller icon art of any kind.
- Repeat place/remove while held: a device-neutral policy, recorded in
  [ticket 29](29-repeat-while-held.md).
- On-screen keyboard, swap sticks and split-screen.
- A virtual mouse cursor.

## Acceptance criteria

- [ ] Play, catalogue select/Shape/Assign to slot, and settings including pad
      rebinding are completable with only a pad (no mouse, no typing).
- [x] In-deadzone drift causes no movement, look or prompt switch; look rate
      is FPS-independent.
- [x] Disconnect, focus loss, menu changes and active-pad switches cancel
      holds and never complete one.
- [x] Keyboard/mouse bindings, preferences and behaviour are unchanged by pad
      use; digital movement speeds are unchanged.
- [x] Pad input is ignored while unfocused; UI navigation buttons and Guide
      cannot be bound.
- [x] Pad crouch toggle and sprint latch let the player crouch-bridge and
      sprint while steering look; neither changes keyboard behaviour.
- [ ] Menus open with focus set, keep focus visible, and adjust sliders.
- [ ] A DualSense shows PlayStation labels; other pads show Xbox labels.
- [ ] Support is claimed only for pad models recorded in a physical
      validation note (model, connection, macOS version, mapping source);
      everything else is marked injected-only.

## Validation and authorization

Headless Bevy tests spawn a `Gamepad` entity and write raw gamepad button/axis
messages. They cover:

- drift inside the deadzone;
- look at 60/144/1000 Hz within one interval;
- trigger hysteresis with no double activation;
- disconnect/focus/menu cancellation mid-hold;
- second-pad takeover only after a deliberate press;
- keyboard/mouse profile byte-identity after pad use and rebinding;
- a profile round-trip including an old file with no `pad` section;
- Server analog magnitude with keyboard speeds unchanged.

Label these injected-only in the validation note.

No physical pad was named during grilling. Build and review on injected
tests; the user runs the hardware checklist before any support claim.

Use the spec's build/review requirements. Preserve shared Bevy artifacts,
Server/core/WIT boundaries and count bounds. Native captures stay in one
unique /tmp folder until review. Local implementation is authorized by this
status; push, publication and merge need separate user instruction.

### Seams with parallel worktrees

- `feat/character-cameras`:
  - Touches `camera.rs::fly`, new `Action` variants (view switch, Zoom In/Out)
    and settings categories.
  - Rebase onto it if it merges first. Keep pad changes to `Axes`, `Bindings`
    and `pad.rs`.
- `feat/world-seed-selection`:
  - Adds the World list and name/seed text fields in `menu*.rs`.
  - Auto navigation covers its buttons, and the text fields are skipped (Q13).
- `server/player.rs`:
  - Either movement branch may change.
  - Keep this PR's edit to the two normalise lines.

## Comments

### 2026-10-27: repository-based triage

Draft scope/acceptance require grilling; do not begin implementation.

### Readiness correction

The user identified that the design had not been grilled. This ticket is a
draft candidate; desktop-only is a platform constraint, not design approval.
Do not implement until the relevant decisions and readiness are established.

### Reconciled after the completed interview

The current spec and What to build section supersede historical draft
alternatives above. Readiness awaits the final shared-understanding check.

### 2026-10-29: controller grilling complete

The user accepted all recommendations Q1–Q21 across three rounds and
confirmed shared understanding. The What to build, acceptance and validation
sections above record them and supersede the earlier draft text. The
grilling found:

- The Server normalised movement direction, so it needs the two-line clamp.
- Rumble is unsupported on macOS.
- Bevy 0.20 already provides automatic directional focus navigation and
  trigger thresholds.

### 2026-10-29: gameplay review against Minecraft and Luanti

A fourth round (Q22–Q28) compared the decisions with Minecraft Bedrock's
controller layout and Luanti 5.17, whose gamepad support cannot drive menus.
The user accepted:

- B toggles crouch on the pad, and LS click latches sprint, because the
  earlier hold mappings were not physically playable.
- A revised Bedrock-aligned mapping, with D-pad up reserved for view switch.
- The menu focus essentials.
- Xbox and PlayStation text label sets.
- Repeat-while-held split out as ticket 29.
- All controller input stays Engine Client, with the Server owning only the
  magnitude clamp.

### 2026-10-29: implementation complete, hardware pending

Implemented on feat/controller and committed locally; see
[PR 9 validation](../pr9-controller-validation.md) for scope, deviations and
injected-only evidence. Ticked criteria rest on headless tests. The three
unticked ones need a physical pad and a visual review, so the ticket waits on
the user's hardware checklist. No support claim is made for any controller.
No push, publication or merge.
