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


## Native menu implementation and Aileron (2026-10-29)

This section supersedes the historical statement above that the approved menu
was not implemented. Tickets 01–04 and 28 remain complete; this UI pass adds no
later-ticket features.

Local implementation batches:

- `92126b4`: leaf-green top-level tabs and native search.
- `bea3b90`: approved layout, typography and vector icon assets.
- `7537a27`: actual Bevy hover subscriptions and binding presentation.
- `1e33fea`: catalogue selection stays open, fixed-size wrapping cards, and
  placeholder/count/spacing refinements.
- `b1e7d02`: public-domain Aileron for menu, keys, values, HUD and save dialogs;
  the reference embeds the same unchanged Regular, SemiBold and Bold faces.

The user selected leaf green and the Mosaic T logo. Catalogue, Behavior,
Key bindings and Display are top-level pages. I opens the last controls page;
the old in-game information button and floating help panel are removed.
Behavior includes an optional quick reference; diagnostics remain independent.
Catalogue and bindings searches preserve text, focus, cursor and page state.
Typing never activates gameplay actions. Draft, conflict, binding capture,
reset confirmation and unsaved-exit dialogs retain the existing input owner
and release-before-activation rules. Save modals retain ownership.

Clicking catalogue content now pins its detail without closing the menu,
changing the hand or grabbing the cursor. Explicit Hold returns to play;
assignment stays in the menu. Real edit-schedule tests exercise solids, fluids
and fire and verify no Edit intent from the selection click. Catalogue cards
are 128 × 113 logical pixels with wrapping columns; native captures verify six
columns at 1280 and five at 1024. Quick slots keep their separate fixed footer.

All 143 Client tests passed after the final implementation, including actual
search widgets, native styles, ownership transitions, ordering and cancellation,
profile failures and 60/144/1000-update intent traces. The native dev executable
also built successfully with warnings denied.

The first final workspace CI run encountered an intermittent failure in
`server::persistence::tests::edits_removals_fluid_fire_and_paused_timers_survive_eviction_and_restart`:
`corrupt World save: invalid simulation Agenda`. The isolated test passed on
recheck. Agenda and persistence implementation/test files are unchanged from
`aa3c2b1` main. The failed run and isolated recheck are retained as
`cargo-ci-aileron.log` and `persistence-recheck.log`; the failure is not claimed
fixed by this UI work.

The full CI recheck passed: 967 tests passed, zero failed, five existing
measurements ignored. Rustfmt, Prettier, all-target/all-feature Clippy,
rustdoc, default-feature checks, benchmark compilation, boundaries and
advisory/license/source/banned-dependency checks passed. The complete log is
`cargo-ci-aileron-recheck.log` in the current review directory.

### Current native review evidence

`/tmp/tesserae-leaf-review-7udu289a/final/` retains twelve offscreen captures,
source revision `b1e7d02`, the executable snapshot/hash, font hashes, per-capture
environment manifests, logs and review hashes. `native-review.html` and
`review.md` at the directory root provide review entry points. Earlier passes
remain retained in sibling directories, including `pre-aileron-and-catalogue`.
The original pre-redesign artifacts remain at the historical path above.

Captures cover Catalogue at 1280 × 900 and 1024 × 900, Material preview and
search, Behavior, bindings and search, Display at 1280 × 900 and 1024 × 640,
and binding capture, reset and unsaved-exit dialogs. Game-provided Materials,
Shapes and textures remain real content rather than the reference's nine
placeholder Materials. Native font rasterization and Game previews differ from
browser rendering; no pixel-identity claim is made across those renderers.

The designer's official Aileron archive and source-page copy are retained in
`/tmp/tesserae-ui-ux-v4-leaf/`. Each selected font's embedded name table states
“No Rights Reserved”; the designer links its public-domain dedication to CC0.
The Liberation binaries and their OFL/author files were removed. Asset
provenance and the approved design record reflect the replacement.

No WIT or Game source changed during this UI pass, and no guest was rebuilt.
The retained CI wrapper follows `scripts/cargo-ci` in order, replacing only
its guest rebuild with compatible-artifact checks/reuse and setting its working
directory explicitly. Cargo used caffeinate, warnings denied and the requested
shared target cache. The branch contains `origin/main` at `aa3c2b1`; nothing
has been pushed, published or merged.


## Direct selection and footer refinement (2026-10-29)

The user's subsequent instruction supersedes the explicit Hold flow above.
Implemented in local commit `eb27484`:

- Selecting catalogue content holds it immediately and keeps the menu open.
  Clicking it again or Clear selection empties the hand. Choosing a Shape
  updates the held variation; closing the menu enables ordinary placement.
- The quick-slot header reads Selected: Stone · Cube (using actual Material
  and Shape names), or Nothing selected. Previewing other content does not
  misreport the held selection. Slot contents are preserved by selection/clear.
- Behavior, Key bindings and Display scroll through all remaining space above
  the bottom divider. Restore defaults is a bordered button; reset confirmation
  and draft-only semantics are unchanged.

All 143 Client tests pass. Actual edit-schedule coverage verifies solids,
fluids and fire, no edit from selection/clear while the menu remains open,
correct rendered summaries, and placement of the selected Shape after closing.
The approved HTML and controls documentation reflect the new interaction.

Twelve offscreen captures and source/executable/font stamps are retained in
`/tmp/tesserae-selection-review-uq4y8tkg/final/`. The review entry point is
`/tmp/tesserae-selection-review-uq4y8tkg/native-review.html`.
Previous captures remain retained at their earlier paths as historical evidence.
Native review confirms the removal of Hold, the selection summary and Clear
selection button, full-height scroll areas, and Restore defaults at 1280 × 900
and the smaller views. Full CI passed at this batch (967 tests, five ignored),
retained as `cargo-ci.log`.


The follow-up `b5a86b9` adds the remappable Empty hand action in Catalogue
(X by default), with a binding-derived footer hint alongside Clear selection.
Actual resolver/edit-consumer tests ensure X does not clear while search owns
input, and works after fresh activation. Cancel and the menu-close shortcut
now join Back, Escape and Catalogue navigation in requesting Save changes /
Discard / Stay. Actual operation and keyboard tests cover all three tabs and
verify persisted settings before closing, while failed saves retain the prompt.
Focused controls/search retain Tab navigation rather than triggering an exit.

The known Agenda persistence failure recurred in `cargo-ci-final.log` before
the final focus guard. It remains unchanged Server code and is not considered
fixed by this UI pass. Final validation uses a recorded four-thread test limit;
this concurrency choice is not evidence that the underlying issue is repaired.


Final validation outcome for the exit/shortcut refinements:

- All 146 Client tests passed, including the Tab focus guard in `71c11ed` and
  the follow-up text-entry guard for a remapped Release cursor action.
- Full CI is **not green**: the unchanged persistence test failed both at
  default concurrency and with `RUST_TEST_THREADS=4`. It passed in isolation
  (`persistence-isolated.log`). No claim of a persistence fix is made.
- `remaining-checks.log` records 969 workspace tests passed, five existing
  measurements ignored, and exactly that one persistence test explicitly
  excluded. Benchmark compilation, boundaries and dependency checks passed.
  The complete CI attempts also passed formatting, all-feature/all-target
  Clippy, rustdoc and default-feature checks before reaching the test failure.
- The final Client-only recheck and native build are retained as
  `client-tests.log` and `native-build.log`. Final lint/check rechecks are in
  `final-lint.log`. Compatible guests were reused throughout.
- Final captures stamp `b5a86b9`; the later focus/text-entry guards change only
  shortcut ownership, without changing their visuals. Earlier `eb27484`
  captures remain in `pre-shortcut-and-exit-guard/`. A separate final native
  executable snapshot, SHA and hash are retained at the review root.

The worktree changes are committed locally; nothing has been pushed, published
or merged. No Server, WIT or Game sources were changed by these refinements.


## Main reconciliation, hotbar restart fix and PR preparation (2026-10-29)

The user authorized resolving main, rebasing this branch and opening a PR.
Local main was clean but diverged by its planning commit `875c7d1`; rebasing it
onto `origin/main` (`aa3c2b1`) produced `8db34f6`. This branch was then rebased
onto reconciled main. The sole conflict was execution-plan documentation;
resolution preserves the confirmed condensed plan and main's conditional
performance gates. Implementation files had no rebase conflicts.

The user then reported loss of hotbar changes after restarting. The explicitly
authorized targeted fix is `3f340f9`: assigned slots, active slot, names and
Orientation are saved atomically per root Game id in a separate hotbar file.
The Server publishes only the stable Game id in the existing internal content
catalogue. No WIT, Game, core, World-save or simulation implementation changed.
Temporary outside-held selections remain transient. Invalid/future files stay
untouched; failed writes preserve the last good file and report an error.
Missing names resolve to empty without being discarded when other slots change.

All 149 Client regressions pass. Three added persistence regressions use actual
catalogue delivery, PostUpdate saving, fresh App instances and temporary files:
restart/reordered ids/orientations, Game isolation/missing-content recovery,
and malformed/future/invalid documents plus backup/write failure and recovery.
The existing Server catalogue regression also verifies publication of Game id.

The retained `cargo-ci-hotbar.sh` follows scripts/cargo-ci, reuses compatible
guests and explicitly excludes exactly the previously documented Agenda test.
That run passes 972 workspace tests, zero failed, with five ignored and one
explicit exclusion. Formatting, all-target/all-feature Clippy, rustdoc,
default-feature checks, benchmark compilation, boundaries and dependency
checks passed. This is not a claim of unfiltered CI success; the existing
persistence failure remains disclosed in the draft PR's checklist.

Logs remain in `/tmp/tesserae-selection-review-uq4y8tkg/`, including
`cargo-ci-hotbar.log`, `hotbar-tests.log` and `hotbar-native-build.log`.
`hotbar-restart/` retains two offscreen native process captures, the shared
saved-layout fixture, source `3f340f9`, binary/hash and environment manifests.
Both launches show the same assigned slots and active fifth slot. The JSON is
a seeded visual fixture; automated tests establish the actual write/restart
behavior. No guest was rebuilt. Cargo used caffeinate, warnings denied and the
requested shared target cache. PR publication is authorized; merge is not.
