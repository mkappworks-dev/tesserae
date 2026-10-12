# Slim dev debug info

Off the build order, so the branch is `tech/slim-dev-debuginfo` with no
`phase-N-` prefix. No tickets: the whole change is two lines in the workspace
`Cargo.toml`.

## Why

A dev build of the workspace leaves a `target/` of 15-25 GB, almost all of it
Bevy and wgpu compiled with full debug info (`debuginfo=2`, the dev default).
Cargo does not share `target/` between checkouts, and this repo is worked from
several worktrees at once, so the cost is paid per worktree: four worktrees
reached 90 GB on top of the 15 GB in the main checkout.

## What changes

`[profile.dev]` gets `debug = "line-tables-only"` and
`[profile.dev.package."*"]` gets `debug = false`.

Line tables keep file and line numbers in panics and backtraces, which is what
is read in practice. Dependencies lose debug info entirely; Tesserae's own
crates keep line tables.

## What is given up

Variable inspection in a debugger. A debugger stepping through Tesserae code can
still set breakpoints and show the call stack, but cannot print locals. Anyone
who needs that for one session can set `CARGO_PROFILE_DEV_DEBUG=2` in the
environment without touching the manifest.

## Not in this change

Sharing one `target/` across worktrees (`CARGO_TARGET_DIR`). That is a
per-machine setting and cannot be committed; a relative `build.target-dir`
resolves per worktree and shares nothing.

## Done when

- `cargo build` still succeeds and `target/` is measurably smaller for the same
  build.
- A panic in a debug build still reports file and line.
