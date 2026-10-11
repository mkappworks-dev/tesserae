# Install Bevy's Linux build dependencies in CI

Status: done
Blocked by: 01

## Task

The `Lint`, `Test` and `Build` jobs fail on Linux with

    The system library `wayland-client` required by crate `wayland-sys` was not
    found.

`wayland` is one of Bevy's **default** features, and `wayland-sys`'s build
script resolves `wayland-client` through `pkg-config` at build time. A missing
`-dev` package therefore fails the build outright rather than degrading to X11.
The workflows installed `libasound2-dev` and `libudev-dev` only — written when
the workspace held `tesserae-core`, which links nothing.

Add `libwayland-dev` and `libxkbcommon-dev` to every job that compiles, in both
`ci.yml` (three jobs) and `build-and-publish.yml` (one).

Do not drop the `wayland` feature instead. Linux is a target platform in
DESIGN.md and Wayland is the default session on current desktops; shipping X11
only would be a platform decision taken to make a CI job go green.

## Done when

All three `ci.yml` jobs pass on Linux, and the release workflow still builds the
Linux target.

## Notes

This class of failure cannot be caught locally on macOS, which links none of
these, nor by `scripts/cargo-ci`. The first Linux build of this workspace is the
PR that introduced Bevy, so it is also the first chance anything could have
noticed.

`boundaries` and `deny` need no packages: `cargo metadata`, `cargo tree` and
cargo-deny read the graph without compiling it.

## Comments
