# 19: Undo/redo and deliberate, content-timed removal

Status: done
Blocked by: 18

User follow-up on 2026-10-27 for PR #21.

## What to build

Keep left removal/right placement in both held and empty hand, but require
a deliberate hold to remove. The user approved about 350 ms with visible
progress and reset on target change, then requested a progress bar over
the block and a Mod-defined time. Add optional remove-time-s to Material
Declarations on unreleased 0.3, validated from 0.05 to 60 s, with a 350 ms
default. Older lines keep that default. Use Server target metadata; do
not raycast or read World storage on the Client. Release, changed target,
menu/cursor changes and invalid aim reset the hold. Each hold removes at
most once. The Server refuses a removal if its intended cell changed.

Ctrl/Cmd+Z undoes and Ctrl/Cmd+Shift+Z (or Ctrl+Y) redoes player World edits.
Keep 128 bounded Server-owned before/after entries. Replay through the
normal change/light/mesh/flow path and publish inverse/forward Mod events.
A new successful change clears redo. Do not rewind simulation or overwrite
a cell changed since the recorded edit, or a regenerated Chunk lifetime.
Refusals leave World state untouched and give visible feedback.

## Acceptance criteria

- [x] Undo/redo restores direct solid, source and fire edits safely.
- [x] Conflicting, unloaded, unlit or regenerated targets are protected.
- [x] Stacks and request queues are bounded; fresh edits invalidate redo.
- [x] Quick left clicks do not remove; progress follows the targeted block.
- [x] Release/target change resets progress; each hold removes at most once.
- [x] Material removal time is validated Game data with a legacy default.
- [x] Server cell validation prevents a delayed hold removing a new target.
- [x] Help, docs, tests, guests, native captures and full CI are complete.

## Comments

The user reconsidered disabling removal while holding a block. The agreed
alternative preserves button meanings and adds a deliberate hold plus
undo/redo. Physical pickup, inventories and object-use menus stay separate.
Undo restores a direct edit's cell only; simulation consequences elsewhere
are not rewound. A changed cell is refused instead of silently overwritten.


Implemented and self-reviewed on 2026-10-27. The Server owns a bounded
128-entry journal and a 16-request queue. Undo/redo uses exact Occupants
and Chunk Lifetime guards, retains refused entries, and publishes the
inverse/forward Mod event through the normal World edit path. No Client
reads storage or mutates World state. Removal timing is optional Material
metadata on unreleased 0.3; 0.1/0.2 adapters retain the 350 ms default.
The gesture timer is Client input behaviour, not network anti-cheat.

Validation: cargo fmt --all, demo rustfmt, npm run fmt and the full
./scripts/cargo-ci pass. 902 tests pass (407 app, 327 core, 69 meshing,
99 Mod), with five existing manual measurements ignored. Guests and the
demo were rebuilt. Clippy, docs, default-feature build, benchmark compile,
boundaries and cargo-deny pass. Earlier Clippy findings were fixed before
the passing run; no new exception was accepted. Core algorithms and bench
inputs are unchanged, so ticket 13's 115-row A/B remains the measurement
evidence. No new benchmark measurement was run for these input/history
changes.

Native offscreen captures were inspected: progress over the selected
Voxel, hidden ghost during removal, centred placement cue and help with
undo/redo. The review folder is retained pending user review:
/tmp/tesserae-p82-removal-review-v41f269b. Its three PNGs are not committed.
The capture fixes progress at 60% using a dev-only rendering seam; input
hold/cancel/once-per-hold semantics are covered by regression tests.
All captures exited successfully with no rendering or validation errors;
macOS logged its existing harmless window-destroy notification at exit.
