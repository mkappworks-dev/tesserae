# Controls and display settings

Open the menu with **Tab**. Its top-level pages are **Catalogue**, **Behavior**,
**Key bindings** and **Display**. **I** opens the last-used Behavior or Key
bindings page; the in-game information button has moved into the menu. Behavior
includes an expandable quick reference, independent of the **F3** diagnostics.

Catalogue searches Material names; Key bindings searches action names,
categories and both assigned alternatives. Search queries stay with their page.
**/** focuses search. Escape clears its text, then releases text focus, before
it can close the menu. Typing in search never activates gameplay or menu
shortcuts. Clicking a catalogue item selects it for placement while keeping the
menu open. Clicking the selected item again, or **Clear selection**, empties the
hand. **X** also clears selection in the Catalogue; the footer hint follows the
remappable Empty hand binding and is inactive while typing in search. Choose a
Shape to hold that variation; close the menu to place it. **Assign to slot**
updates a shortcut. The quick-slot header shows the current selection, such as
**Selected: Stone · Cube**, or **Nothing selected**. Catalogue tiles are 128 ×
113 logical pixels and wrap into as many columns as the window fits. Behavior,
Key bindings and Display scroll through all the space above the footer divider;
**Restore defaults** is a button in that footer.

Each keyboard/mouse action has two alternatives. Behavior, binding and display
changes share one draft across their pages. **Apply changes** saves and
activates it only after a successful write. Closing through **Cancel**, Back,
Escape, Tab, or leaving for Catalogue with edits offers **Save changes**,
**Discard** and **Stay**. A failed save keeps the draft and prompt open. Restore
defaults asks for confirmation and changes only the draft. Escape always cancels
binding capture. Capture waits for the opening inputs to be released.

Conflicting bindings offer Replace or Cancel. Modifier sides are equivalent for
chords; physical modifier keys can also be bound individually. Specific chords
consume their trigger before plain actions. History chords also suppress
descent. Redo takes precedence over undo; empty hand suppresses sampling;
placement, sampling, selection and history cancel pending removal.

**F5** switches between first-person and an over-the-shoulder third-person view
in either movement mode; it never moves the Player. **Alt+wheel** zooms the
third-person camera from 1.5 to 6 m. Wheel directions can be bound like keys,
with or without a modifier; a bound wheel chord owns the wheel, so it never also
steps the quick slots, and plain wheel stays slot selection. The view and zoom
are saved in the user profile, not in any World.

The crosshair, the targeted-Voxel outline, the placement preview and breaking
are for building: they show only while the hand holds something (a block, a
fluid, fire or a tool) or Layout mode is on. With an empty hand you only look.

**L** opens and closes the Layout card, and Layout mode stays on while it is
closed. Place commits a layout, Remove cancels it and the clear key
(**Backspace**) leaves the mode. **]** and **[** change the count on the active
axis, digits type it, **J** moves to the next axis and **T** reverses it. Point
to point takes A, B, then C for depth; C needs no surface. These are remappable
actions under Key bindings.

Gameplay input is cancelled when ownership changes. Inputs held through a menu,
focus change or device cancellation require release before activation. Pending
sampling replies are discarded. The Server continues simulating while UI is open
and remains the sole authority over Edits and the Player's pose.

Hold sprint and hold removal are the defaults. Toggle sprint survives stops, but
clears on UI entry, focus loss and respawn. Its indicator is visible while
enabled. Timed tap removal uses the Game's Material duration, stays on one
original target, and completes once. A second tap cancels it. Invalid aim, a
lost or changed target, or another selection/edit/history action cancels it
without restarting.

**Use** (**G**, or D-pad up) asks the Server to use the aimed Voxel with a held
tool. **M** cycles the declared tools, past the last returns the previous hand,
and **Backspace** puts the tool away. While a tool is held, Place and Remove are
the tool's verbs and Edits are inert; undo and redo stay live. One Use per
press, never repeated while held, and a Use never enters the undo journal. A
refusal raises the usual alert; success raises none. A tool is never stored in a
slot or saved.

**Repeat while held** is a Behavior preference, off by default. With it on, a
held Remove starts a fresh timed removal on the next target after each one
completes, and a held Place sends its first intent on press, then another every
0.2 s after 0.25 s. Only solids repeat; fluids and fire never do. A refused
repeat is silent. Releasing, losing the target, opening a menu or changing
device, hand, Shape or orientation stops the chain.

Underwater, a submerged camera swaps to a short-range fog and tint in the
fluid's colour. In physics movement a body floats, sinks and swims in a fluid,
and Jump held swims up; flight ignores water.

## Controller

A gamepad plays through the same actions as the keyboard. The sticks move and
look; menus use fixed focus navigation with no virtual mouse. Pad bindings are a
separate set in the same profile, two alternatives per action and no modifiers,
and are checked for conflicts only against other pad bindings.

| Input               | Default action                  |
| ------------------- | ------------------------------- |
| Left stick, click   | Move, Sprint (latched)          |
| Right stick, click  | Look, Sample                    |
| South (A), East (B) | Ascend, Descend (crouch toggle) |
| North (Y), West (X) | Catalogue, Shape                |
| Right trigger, left | Remove (hold), Place            |
| Bumpers             | Previous, next quick slot       |
| D-pad left, right   | Turn, Flip                      |
| D-pad down, up      | Empty hand, Use                 |
| Start, Select       | Help, Movement mode             |

Pad look sensitivity, invert Y, both deadzones (0.15) and crouch toggle are pad
preferences, separate from the mouse ones. The Guide button is never bound. Only
the pad models recorded in the validation notes are claimed to work.

## Touch

Touch is for desktop touchscreens on Windows and Linux. winit never delivers
touch events on macOS, so macOS is unclaimed. Each contact owns one role from
start to end: Move, Look, a button, or UI. A contact that slides off a button
still belongs to it, and extra contacts beyond four are ignored. Touch bindings
are fixed and not remappable. Touching the world shows the overlay; deliberate
key, mouse or pad use hides it. Mouse events that follow a touch within 0.3 s
are treated as the OS's emulation of it and ignored.

The overlay has a stick on the left, look on the right, and buttons for Place,
Remove (hold), Use, Sample, Ascend, Descend, Sprint, slots, Shape, Turn, Flip,
empty hand and view. A Menu button sits in a top corner. Targets are at least 44
logical pixels with 8 pixel gaps. Touch preferences cover look sensitivity,
invert Y, a mirrored layout, size (small, normal, enlarged) and opacity.
Catalogue and menus work without hover or drag: tap selects and a separate
**Assign to slot** control assigns. A drag past 6 pixels scrolls and never
selects. Text entry uses the physical keyboard. Touch is implemented and tested
with injected events but not yet validated on a real touchscreen.

Mouse sensitivity, invert Y, scroll direction and scroll sensitivity are user
preferences. Labels use observed logical keyboard events separately from the
serialized physical key identifiers. Keys without an observed logical event use
a readable physical-key fallback. Changing a keyboard layout does not silently
relocate bindings.

The version 1 user profile defaults to:

- macOS: `~/Library/Application Support/Tesserae/user-profile.json`
- Linux: `$XDG_CONFIG_HOME/Tesserae/user-profile.json`, or
  `~/.config/Tesserae/user-profile.json`
- Windows: `%APPDATA%/Tesserae/user-profile.json`

`TESSERAE_USER_PROFILE` overrides the path. New actions merge their defaults. A
new default that conflicts with an existing user binding stays unbound rather
than displacing that binding. Invalid or unsupported profiles remain on disk and
loading reports an error. Apply preserves the previous valid profile as
`.json.bak`, or preserves corrupt data under a timestamped `.corrupt-*` name.
Saves write and sync a temporary file, then atomically replace the destination.
Save failure keeps the active preferences and draft distinct. Profiles contain
no World data or Material ids.

Assigned hotbar slots and the active slot are saved automatically per Game in
`user-profile.hotbar.json` beside the controls profile (the overridden controls
profile filename determines this filename too). Stable Material and Shape names
and orientations survive registry reordering. Missing content appears empty; its
saved name is retained when other slots change, so reinstalling content can
restore it. Temporary outside-held content is not part of the saved layout.
Invalid or unsupported hotbar files are retained untouched; save failures are
reported and preserve the previous valid file. Hotbar storage is independent of
the Behavior, Key bindings and Display draft and contains no World data.

Normal play defaults to display sync. FPS caps and explicit uncapped rendering
change presentation pacing only; they do not change quality, view distance or
Server simulation rules. Capture fixtures retain their explicit uncapped policy
and record that override. A synced or capped FPS counter does not measure
renderer capacity.

Profiling records selected presentation preferences, requested window mode,
focus, physical camera targets and observed backend surface configurations. The
backend observations come from wgpu's concrete surface configuration events,
including mode and dimensions, rather than an inference from `AutoVsync` or
`AutoNoVsync`. They require the `wgpu_core::device::resource=debug` log
directive; `scripts/profile-frames` sets it. Missing surfaces or filtered logs
leave actual configuration unavailable. Observation timestamps are seconds since
the observer was created.
