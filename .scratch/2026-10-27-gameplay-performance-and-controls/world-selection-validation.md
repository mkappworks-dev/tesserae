# PR 7 (W0) implementation and validation

Scope: ticket 27 only. Base: main 96ea33e. Branch `feat/world-seed-selection`
in `.worktrees/feat+world-seed-selection`. No push, PR or merge.

## Commits

- 28a782f: ticket 27 scope after grilling Q1–Q24; W1 and G0 stubs.
- 1aef416: `fbm2` wraps octave seeds; pinned digests from main prove existing
  seeds keep their terrain at generator version 1.
- 265c279: `tesserae_core::seed::resolve`, golden vectors from `shasum`.
- 365fbe4: `save::library`: readable folders, sidecar, list, rename, the
  writer-lock rule and adoption of the pre-list `default` World.
- 8c7a45f: `Opening::from_folder`, saved-settings precedence, Empty World
  loads no Mods, last-played, pending cleared after the first commit.
- 1ce5149: World list, Create/Re-Create/Rename/Delete/Trust, re-exec
  switching, Return to World list, loading screen.
- 98e5952: native fixes: relaunch handoff out of the consumed `App`,
  logical window size, Create actions visible at 1024 x 640.
- 146e400, df3c06c, ec2b4ae: docs and review fixes (below).
- 48ba6d8: dev-only list captures render offscreen on a locked display.

## Automated evidence

`scripts/cargo-ci` passed on ec2b4ae with RUSTFLAGS="-D warnings" and the
shared target: rustfmt, Prettier, all-feature Clippy, rustdoc, default
build, guests, workspace tests (1013 passed, 0 failed, 6 ignored), bench
compilation, boundaries and dependencies (cargo-deny ok with `trash`).
48ba6d8 changes only dev-only capture code; all-feature Clippy and the dev
build passed on it afterwards.

Seams agreed before tests: core seed/noise, core World library, Server
Opening, Client launcher app. Notable regressions:

- Seed: blank, unsigned/signed decimal, `u64::MAX`, `i64::MIN`, out of
  range, words, Unicode; blank shows and uses a pre-drawn random seed.
- Generation: `fbm2` at `u32::MAX`, boundary sweep, fold equivalence,
  digests for three seeds recorded against main's `noise.rs`.
- Library: same-name folders and UUIDs, OS-safe names, case-only
  collisions, newest-first list hiding no folder, rename touching only the
  sidecar, repair from the commit, Busy refusal while open, adoption.
- Opening: pending identity/settings, 0.25 m reopen against a 1 m
  environment, same-name/seed isolation after edits, details without a save
  refused, missing content/lock/corrupt commits leave bytes unchanged,
  unreadable details never block a saved World, Empty World.
- Launcher: create journey and restart, invalid input writes nothing,
  Cancel-focused Delete refused while locked, Trust before open with Cancel
  running nothing, Empty World skips Trust, failure card, rename and
  Re-Create (same Game only), Quit, Return to World list routing.
- Launch: argument parsing, direct-variable bypass, geometry round trip,
  handoff outliving the `App`.

## Native evidence

macOS, Apple M5/Metal, dev binary of 48ba6d8 with SHA-256
0f74276b6c7e81132cee4aeea000be1da12d239060de9a5b217e6b314748fec5, isolated
`HOME` under `/tmp/tesserae-w0-review`. The display locked mid-session, so
final captures use the offscreen path; earlier window captures of 98e5952
matched them. Captures (not committed): empty list, Create with World
options, populated list, reopened World, failed open of a corrupt commit
(save bytes compared unchanged), Delete confirmation, Trust dialog with
`--mods`, Create and list at 1024 x 640.

The real create journey ran natively: the list created `New World`, the
process re-executed into it, the first commits landed with `pending`
cleared, and a later `--world` launch logged the same UUID as reopened.

## Deviations and limits

- A World without readable details lists as "Unnamed" with its folder
  name, not its UUID: reading the UUID means decoding a commit of up to
  256 MiB. Opening still works from the commit.
- The list process has no input owner, so its fields use Bevy text input
  directly rather than `Owner::TextEntry`; no gameplay input exists there.
- The loading screen appears once the World process has a window. Mod
  loading and save validation run before that, so a large Game shows no
  window for that time. Moving them behind a window is W1-sized work.
- Window position carries over only after the window has moved (winit
  reports none before); size always carries. macOS native fullscreen is
  not reflected in Bevy's `WindowMode`, so it is not carried.
- An open failure reports its final reason; a successful fallback to an
  older commit shows the in-World banner instead of list diagnostics.
- No input automation is available, so Tab, Enter, Return to World list,
  Copy and Show folder were not exercised natively. Tests cover Escape,
  activation and the routing; Tab relies on Bevy's TabNavigationPlugin.
- No single end-to-end test spans Create, Rename and reopen with edits;
  each step is covered at its seam.
- Windows/Linux runtime, Trash and file-manager behavior are not tested
  here. User visual review is pending.
