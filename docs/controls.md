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
