# PR 2 implementation and validation

Local work for tickets 01–04 and 28. No push, publication or merge.

## Local batches

- `11c192e`: confirmed condensed plan and ticket authorization.
- `4ca94b4`: semantic input, ownership, profile, controls and display settings.
- Follow-up changes add backend configuration observations and validation fixes.

## Implemented

Gameplay consumers share action frames and explicit ownership. Menu/cursor
changes precede gameplay consumers. Context transitions block held inputs
until release. Pending sample replies are discarded before application.
Settings/capture and text-entry/device-cancellation seams share this policy.
Server authority and simulation bounds are unchanged.

Two keyboard/mouse binding alternatives, conflict replacement, release-armed
capture, logical labels, default history alternatives, draft Apply/Cancel,
unsaved exits, per-action/full reset, versioned atomic profile persistence,
sprint toggle, timed tap removal, mouse/scroll preferences and display policy
are implemented. Normal play defaults to display sync; capture fixtures keep
an explicit uncapped override. FPS caps pace presentation independently of
World settings. Backend metadata observes wgpu's concrete configuration event.

## Checks so far

- Client suite: 119 passing tests, including cue/context and alternative-edge changes.
- Workspace all-target/all-feature Clippy passed; final follow-up rerun pending.
- Native settings rendering and profiler metadata ran successfully offscreen.
  Concrete backend configuration recorded `present_mode: Immediate`,
  2560 x 1440 surface dimensions; camera output was separately offscreen.
- Actual-consumer regressions cover menu ordering, held respawn/movement,
  sampling cancellation, toggle sprint and Material-timed tap removal.
  Action and removal traces cover 60/144/1000 updates per second.
- `scripts/cargo-ci` ran with its guest-build step replaced by cached-artifact
  reuse. Lint, docs and default-feature checks passed. Workspace tests stopped
  with 396 passed, 18 failed, two ignored in the application crate.
  All 18 failures concern incompatible demo/lake guest artifacts: the cached
  components expose a four-field record where this branch expects two.
- The two Python profiling-script tests passed with warnings treated as errors.
- No WIT/Game source changed. Guest rebuilding remains pending the user's
  answer to the permission question; no guest rebuild has been performed.

## Retained native artifacts

`/tmp/tesserae-controls-review-jEORfd` contains native PNG captures, logs,
profile CSV/metadata, an executable snapshot, cached-guest hashes and the
CI wrapper/log. Early captures used an empty World; they validate settings
rendering rather than Game content or completed removal gestures. The
incompatible cached Game prevents those native content captures until resolved.
These are functional captures, with no performance-improvement claim.

Final CI completion, remaining acceptance review and final ticket status are
still pending. Do not treat the local commits as a completed PR 2 acceptance.
