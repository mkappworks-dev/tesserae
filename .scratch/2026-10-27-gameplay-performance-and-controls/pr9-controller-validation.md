# PR 9: controller input, navigation and settings

Scope: ticket 07 only. Base: 96ea33eaf6fdea8d2c23f8c751852736c0394107.
Branch/worktree: feat/controller. Local commit only; push, publication and
merge are not authorized.

## Implemented

- Bevy 0.20's built-in gamepad support (gilrs via default features). A new
  client/pad.rs reads one active pad into PadInput: buttons for actions, all
  buttons for menus, conditioned sticks and a menu cancel edge.
- Trigger::Pad joins the shared resolver. Bindings holds separate keyboard and
  pad maps, two alternatives each. The profile adds a pad_bindings section
  and preferences.pad, both serde-defaulted under version 1. Mode/Guide,
  modifiers on pad bindings and cross-device entries are refused on load.
- Defaults follow ticket 07's table. PreviousSlot/NextSlot are new actions
  (LB/RB on the pad, unbound on the keyboard).
- Sticks use a scaled radial deadzone (0.15 default); look adds a 1.5
  exponent and turns at 3 rad/s times sensitivity, integrated over time.
  Trigger hysteresis is press 0.6 / release 0.4 via GamepadSettings.
- Pad crouch toggles on foot and holds in flight; LS click latches sprint
  until the stick rests or crouch engages. Keyboard hold behaviour unchanged.
- Server: movement direction length, capped at 1, is a throttle. The Client
  sends keyboard directions at unit length. ADR-0008 gains an Analog throttle
  section.
- Menus: d-pad/left stick move focus with repeat, South activates, East and
  Menu cancel like Escape, LB/RB cycle pages, sliders step on left/right.
  Focus starts on the held Catalogue entry, returns near its last place after
  a rebuild, stays inside modal dialogs and scrolls into view. Search fields
  are skipped. Focused buttons, tiles, empty bindings and slider thumbs show
  the hover highlight.
- Settings gains a Controller page: look sensitivity, invert Y, both
  deadzones, crouch hold/toggle and pad binding rows. Restore defaults on
  that page resets only controller state; elsewhere it keeps it.
- Prompts follow the last deliberate device. Sony (vendor 0x054C) pads show
  PlayStation names; others show Xbox names. A held stick claims the prompts
  once, so a key pressed while steering takes them back.
- Unfocused windows ignore pads. A new pad takes over only on a deliberate
  press, which it does not spend. Losing the active pad cancels holds.
- In PointerFree, South or closing a menu with the pad returns to play.

## Deviations from the ticket

- Only Menu (and Escape) cancel a pad capture; East is capturable, since it
  is Descend's default and could otherwise only be restored by reset.
- The Server change is not a literal clamp in both branches: a literal swap
  slowed walking while looking down. See ADR-0008.
- Navigation calls bevy_input_focus's find_best_candidate over Button and
  Slider entities instead of tagging them with AutoDirectionalNavigation,
  which is z-index agnostic and would leave modal dialogs.
- D-pad up is unbound by default but capturable.

## Validation

Headless tests are injected-only; physical results are in Hardware results
below and are recorded separately.

Headless tests drive Gamepad entities through Bevy's raw gamepad messages or
PadInput directly. They cover: trigger hysteresis with one activation per
press; drift producing no movement, look or prompt switch; disconnect, menu
and focus loss cancelling a hold without completing it; second-pad takeover
spending no press; unfocused windows; menus withholding navigation buttons
and turning them into cancel; a navigation button held through a menu close
waiting for release; crouch toggle/hold and sprint latch; look at 60/144/1000
Hz within one 60 Hz frame; LB/RB slot stepping; pad capture and per-device
reset; profile round-trip, pre-controller profiles and refused entries;
Server analog throttle at 1, 0.5 and 0.25 m Voxels.

Final scripts/cargo-ci (after cadaf43) passed: 992 tests, zero failures,
six ignored measurements; formatting, all-feature Clippy, rustdoc,
default-feature build, guests, headless/Mod boundaries and cargo-deny also
passed.

Artifacts: /tmp/tesserae-controller-review.OjvT3b (cargo-ci.log).

## Hardware checklist (user)

Record model, connection, macOS version and gilrs mapping source for each pad.
Then confirm: default mapping, stick feel and deadzones, triggers, LB/RB slots,
crouch/sprint latches, full catalogue select/Shape/Assign to slot and settings
rebinding without mouse or keyboard, PlayStation labels on a DualSense,
hot-plug mid-hold, and that Guide opens the macOS overlay without game input.

## Hardware results

Run by the user on 2026-10-29 with the dev binary from cadaf43 (SHA-256
prefix 3d166f6a74c0efd6), macOS 27.2 (26B123), Apple M5. Controller
families: Xbox, DualSense/DualShock, and Switch Pro/8BitDo, over both
Bluetooth and USB. Exact models and gilrs mapping sources were not recorded.

The user reported every checklist step passing. The first run panicked on
opening the Controller page (binding_order lacked MovementMode, also broken
on main's Key bindings page since #26); cadaf43 fixed it and the user re-ran
the menu steps successfully.

Support claim: those three families, wired and Bluetooth, on this macOS
version, at family level only. Nintendo-layout pads show Xbox names by
position, so their printed A/B are swapped relative to the label; dedicated
Nintendo labels remain out of scope. Other platforms and models are
unvalidated.

