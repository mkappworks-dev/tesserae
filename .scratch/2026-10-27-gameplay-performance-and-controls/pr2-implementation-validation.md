# PR 2 implementation and validation

Tickets 01–04 and 28 are implemented locally. No push, publication or merge.
Source base: `b7997b025a4f31e959ed090e23469f7665820670` (PR 1, #22).
Fetching and rebasing onto `origin/main` found the branch already up to date.

## Local batches

- `11c192e`: confirmed condensed plan and ticket authorization.
- `4ca94b4`: semantic input, ownership, profile, controls and display settings.
- `19ee970`: backend configuration observations and transition validation.
- `07a7f41`: reserved Escape cancellation after rebinding.
- `acd469d`: active-alternative ownership, reset conflicts and profile merging.
- `964caea`: one-shot cancellation, text entry, redo and timing regressions.
- `869d999`: shared look/wheel axes and focused free-cursor UI shortcuts.

## Implemented

Gameplay consumers share action frames, look/wheel axes and explicit ownership. Menu/cursor
changes precede gameplay consumers. Context transitions block held inputs
until release, including when an unused alternative exists. Shared axes are
discarded in UI contexts; a focused free cursor retains help/diagnostics. Pending sample
replies are discarded before application. Settings, capture, text-entry and
device-cancellation contexts share this policy. Server authority, aim/history
guards, logical simulation and count bounds are unchanged.

Two keyboard/mouse alternatives, specific-chord consumption, conflict
replacement, release-armed capture, logical labels and default history
alternatives are implemented. Apply saves and activates the draft; Cancel
keeps the previous settings. Unsaved exits offer Apply/Discard/Stay. Individual
and full resets follow the same conflict rules. Escape remains a reserved
cancellation route even after rebinding cursor release.

The versioned profile uses atomic replacement and preserves prior/corrupt
files. New defaults cannot displace existing user bindings. Save failure keeps
the active settings and draft distinct. Controls contain no World state or
runtime Material ids. Logical keyboard-event labels are separate from physical
serialized ids, with a physical-key fallback before a label is observed.

Hold modes remain defaults. Toggle sprint survives stops and clears on UI,
focus loss and respawn. Timed tap removal stays on one Server-resolved target,
uses the Material duration, completes once and cancels on a second tap,
invalid aim or another relevant action. Hold removal retains held-place
priority. Sensitivity, invert Y and scroll preferences use the same profile.

Normal play defaults to display sync. Caps pace presentation independently of
World settings and quality. Capture fixtures retain their explicit override.
Profiler metadata distinguishes selected policy, requested window mode and
observed concrete backend configuration, with focus and physical targets.
Native metadata observed both Fifo and Immediate, rather than inferring the
backend mode from AutoVsync/AutoNoVsync.

## Validation

- Full `scripts/cargo-ci` passed after repairing the incompatible guests.
  Its guest-build step was replaced by reuse of existing artifacts; all other
  checks ran in their original order: 925 tests passed, zero failed, five
  existing measurements ignored. The final committed-state rerun is
  recorded in `cargo-ci-shared-frame.log` under the artifact directory below.
- Workspace all-target/all-feature Clippy, rustdoc, default-feature check,
  formatting, benchmark compilation, boundaries and dependency checks passed.
- All 129 Client regressions passed. Acceptance coverage includes actual
  menu/movement/respawn ordering,
  held alternatives, pending sampling, profile failures and reset conflicts.
  Non-US logical labels are tested with injected keyboard events, independently
  of physical ids; this is not a physical keyboard-layout certification.
- 60/144/1000-update traces cover action edges, one-shot device cancellation
  and actual timed-removal intents. Completion allows one render interval of
  quantization, never early removal, repeats or target transfer. Both modifier
  sides and Ctrl+Y / Ctrl-or-Command+Shift+Z redo alternatives are covered.
- Four profiler regressions passed, including the backend event observer.
  The two Python profiling-script regressions passed with warnings as errors.
- Cargo used `caffeinate -i`, `RUSTFLAGS='-D warnings'` and the shared target at
  `/Users/mk/.cache/cargo-target`, with workspace/dev where applicable.

## Guest artifact repair

The initial cached demo/lake exposed a four-field WIT record while this branch
expected two, causing 18 Server tests to fail. The user explicitly authorized
rebuilding only those two artifacts. Cargo initially reused its stale output;
forcing those two guest compilation units to recompile repaired the mismatch.
Their dependencies and all other guests were reused. No WIT or Game source
contents changed. Repaired hashes are in `rebuilt-guests.sha256` and the final
artifact inventory. Earlier failed logs are retained as superseded evidence.

## Native review artifacts

The last native captures use code revision `869d999`; the source SHA and
executable hash are retained beside them.

All captures, logs, raw profiles, metadata, environment manifests, executable
snapshot and hashes remain under:

`/tmp/tesserae-controls-review-jEORfd`

- `default-settings.png`: default controls and display-synced policy.
- `tap-toggle-settings-small.png`: 1024 x 640 settings with optional modes.
- `tap-progress-sprint-visual-fixture.png`: visible sprint indicator and a
  half-filled removal bar on a Server-resolved target within ordinary reach.

Captures render offscreen and use the real demo, unchanged quality and reach.
The progress/sprint image deliberately injects presentation state through
capture-only fixture hooks. It is visual evidence; the intent regressions,
not that fixture image, establish gesture timing and cancellation correctness.
Early empty-World and preliminary captures remain separately retained. Native
visual acceptance remains available for user review. No performance speedup,
GPU elapsed-time or display-latency claim is made.


## Rebase onto saved-World main (2026-10-28)

Rebased onto `aa3c2b1` (PR #23). The earlier commit identifiers, test counts,
and native captures above describe the pre-rebase implementation.

Preserved main's save plugin and playing gates on camera and history consumers.
Save dialogs now own input, cancel active gestures and binding capture, and
block settings activation while retaining the draft. Held gameplay bindings
require a fresh activation after the modal closes. Two added regressions cover
the actual resolver and settings consumers; all 133 Client tests passed.

Full CI passed after the rebase: 957 tests passed, zero failed, five existing
measurements ignored. Formatting, all-target/all-feature Clippy, rustdoc,
default-feature checks, benchmark compilation, boundaries and dependency checks
passed. The CI wrapper retains the script's order and checks, replacing only
its guest rebuild step with artifact reuse. Main changed WIT 0.3 world-info;
only its incompatible lake and demo artifacts were rebuilt. Other guests were
reused. No WIT or Game source contents changed in this branch's integration.
Cargo ran under caffeinate with warnings denied and the shared target directory.

Rebase CI logs, wrapper and rebuilt guest hashes are retained in
`/tmp/tesserae-ui-ux-v3-65IqS3`. The native captures above remain historical;
this rebase was validated with automated checks, without new native captures.
The refined menu, logo and accent studies in that directory are design
prototypes; the visual redesign has not been applied to the native UI.
