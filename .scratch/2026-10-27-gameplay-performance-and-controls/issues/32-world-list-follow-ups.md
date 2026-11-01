# 32: W0.1: World list window and controller follow-ups

Status: done
Blocked by: none (27 is merged as #30)

Spec: [reconciled desktop gameplay](../spec.md).
PR boundary: W0.1; keep this separate from other PR scopes. One PR on
`feat/world-list-follow-ups`, titled
`feat(world): World list window and controller follow-ups`.

## What to build

Split from ticket 27's acceptance criteria on 2026-10-29. Scope settled in the
2026-10-30 grilling below.

### Window position

- winit sends no position until the first move, so Bevy's `WindowPosition`
  stays `Automatic` for a window that never moved. Read the live position
  from winit (`WinitWindows`, `outer_position()`) in `launch::track` whenever
  Bevy's value is not `At`, and hand it off as today. No new crate and no
  `objc` calls.
- Where winit returns `Err` (e.g. Wayland), keep centring and record that in
  the ticket as accepted.
- Positions stay physical pixels and size stays logical. Do not re-scale
  across monitors and do not store a monitor id or name.

### Restoring safely

- Add a pure `fit(geometry, monitors) -> Geometry`. A saved top-left outside
  every current monitor (display unplugged, resolution changed) is dropped
  and the window centres. A size larger than the monitor under it is clamped
  to that monitor.
- Monitors are not known when `window_plugin` builds the `Window`, so apply
  `fit` on the first frame after the window exists. The one-frame jump is
  accepted; note it in the validation note.
- A malformed `--window` argument still falls back to the default window.

### Native fullscreen

- Bevy's `WindowMode` ignores the macOS green button, but winit's
  `fullscreen()` reflects it. Read it in the same `track` system and map
  `Some(_)` to `Mode::Borderless`. Confirm winit's macOS `Borderless` is the
  native `toggleFullScreen`; if it is not, stop and report, do not add
  Spaces-specific code.
- A fullscreen restart keeps the last windowed size and position, not the
  fullscreen frame.

### Controller on the World list

- `list_app` in `main.rs` adds only `launch::plugin` and
  `client::worlds::plugin`, so ticket 07's `client::pad` (`navigate`,
  `Ownership`, `Labels`, `Settings`) never runs there. This is a wiring gap.
- Reuse `pad::navigate` first, adding the minimum shared resources to
  `list_app`: d-pad or left stick moves focus, South activates, East is Back
  (Escape's action: Dismiss on the list, Back elsewhere), focus is shown after
  pad use. Keep these fixed and not remappable. If `navigate` is too tied to
  `Settings`, `Labels` and `Ownership` to reuse without touching play code,
  use a small launcher-local adapter with the same bindings instead. Either
  way, change no `client/pad.rs` semantics.
- Every journey completes with the pad alone and no typing: Open, Create
  (default name "New World" plus Randomize), Re-Create, Rename, Delete, Trust,
  Show World folder and Quit. Directional navigation skips text fields. Rename
  cannot be edited by pad, only confirmed or cancelled; record that. No
  on-screen keyboard.

### Accidental actions

No new protection beyond what exists (Delete opens with Cancel focused and
styled destructive). Ensure:

- A pad button held when a screen opens is ignored until released, so the
  South press that chose Delete cannot confirm it. Reuse ticket 07's held-
  button rule if `navigate` is reused; implement it in the fallback adapter.
- Opening or creating re-execs, so no held button carries into the next
  process.
- Pad input is ignored while the window is unfocused, as in ticket 07.

### Native keyboard check

Tab, Shift-Tab and Enter cannot be automated here. Add tests for focus order
per screen and Enter activating the focused button. The user does the native
pass on the list, Create, Rename, Delete and Trust screens, and the result
goes in a short validation note beside the ticket. Position and fullscreen
restarts are recorded there too. The ticket stays open until the note exists.

### Out of scope

Ticket 30 (W1) deletes this window code, so do not generalise it: no
persisted window config, no per-monitor profile, no `States`, no loading
screen, no change to re-exec or `Handoff`. No Server, `Commit`, WIT or
play-time controller change, and no demo or performance work. Touch only
`launch.rs`, `main.rs` window setup, `client/worlds.rs`, the list app's pad
wiring and tests.

## Acceptance criteria

- [ ] Restarting from the list into a World, and back, keeps a never-moved
      window's position. Where winit cannot report one, centring is recorded
      as accepted.
- [ ] A restart from macOS native fullscreen stays fullscreen. Windowed size
      and position are the last windowed ones.
- [x] A saved position outside every current monitor, or a size larger than
      its monitor, falls back to centred and clamped. A pure function is
      unit-tested for a removed display, a negative-origin second monitor and
      an oversized window.
- [x] A controller completes Open, Create (default name, Randomize),
      Re-Create, Rename (confirm or cancel), Delete, Trust, Show folder and
      Quit with no mouse and no typing, through ticket 07's navigation.
      Tested headless for Create, Delete, Rename, Trust and Back; Open,
      Re-Create, Show folder and Quit use the same activation path.
- [x] A pad button held while a screen opens is ignored until released, so
      the South that chose Delete cannot confirm it. Pad input is ignored
      while the window is unfocused.
- [ ] Tab, Shift-Tab and Enter are verified natively on the list, Create,
      Rename, Delete and Trust screens, with a validation note.
- [ ] The validation note lists the physical pad used (model, connection,
      macOS version), per ticket 07's support rule, and marks everything else
      injected-only.

## Validation and authorization

Behavior/intent regressions plus the spec's build/review requirements.
Headless tests for `fit`, pad journeys on the list app, and the held-button
rule. Native captures stay in one unique /tmp folder until review.
Implementation, push, publication and merge need separate user instruction.

## Comments

### Grilling 2026-10-30 (Q1-Q12, all accepted)

Q1-Q2 read position and fullscreen from winit in `launch::track`. Q3 reuses
ticket 07's `navigate` on the list, with a local adapter only as a fallback.
Q4 and Q10 keep the PR inside the window and list code so ticket 30 is not
pre-empted. Q5-Q6 add a pure `fit` against current monitors and nothing
per-monitor. Q7-Q8 fix the pad journeys as typing-free and add only a
held-button guard. Q9 keeps the native Tab/Enter pass as a user step with a
validation note. Q11-Q12 set these criteria and limit edits to this ticket
plus one line in ticket 30.

### Implementation 2026-10-30

Implemented on `feat/world-list-follow-ups`. Position and fullscreen are read
from winit in `launch::track` (`observe`), and `launch::restore` fits the saved
geometry to the monitors on the first frame (`fit`). The list app now runs
`pad::launcher`, which reuses ticket 07's `read` and `navigate` under one
standing modal owner, and East or Start goes Back like Escape. Full cargo
test, clippy, fmt and doc pass once `./scripts/build-guests` has built the
demo; without it 59 server tests fail for want of `games/demo/dist`.

Two defects turned up while testing Tab and the pad, both fixed here:

- The empty list's "No Worlds yet" card was a modal `TabGroup` with no
  controls, so Tab and pad focus could never reach Create World. Only dialogs
  trap focus now.
- The list screen had no `TabGroup`, so Bevy's tab navigation returned
  `NoTabGroups` and Tab did nothing there. The list root has one now.

The held-button rule holds by construction (`navigate` acts on
`just_pressed` only) and is locked in by tests; no gate was added.

Still open, so the ticket is not done: the native pass (Tab, Shift-Tab, Enter
on the five screens), a restart that keeps an unmoved window's position, a
restart from native fullscreen, and the physical pad note. winit's macOS
`Borderless` is the native `toggleFullScreen`, and the green button sets
`Borderless` in winit's own state, so `fullscreen()` reflects it.

Accepted limits, recorded as the ticket asked:

- Where winit cannot report a position (`outer_position()` is `Err`, as on
  Wayland) the window centres.
- `restore` runs one frame after the window exists, so a fitted window jumps
  once.
- Rename cannot be edited by pad, only confirmed or cancelled; there is no
  on-screen keyboard.
- `fit` measures an unplaced window against the first monitor Bevy lists.
- The list uses default pad deadzones, not the saved profile's, because no
  profile exists there.

Code review found the fullscreen windowed frame survived only one restart,
since `LastGeometry` started empty; it now starts from the launch geometry.

