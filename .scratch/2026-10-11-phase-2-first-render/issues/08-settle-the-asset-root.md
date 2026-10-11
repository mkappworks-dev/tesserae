# Settle where `assets/` lives and how it is found

Status: done
Blocked by: 07

## Task

Ticket 07 left `bevy/file_watcher` off because there was no `assets/` directory
and no decision about where one ships. Make the decision.

`docs/architecture.md` already puts `assets/` at the repo root, next to
`crates/` — the layout was written before any Bevy code existed and nothing has
contradicted it. Keep it there and make the engine actually find it.

Bevy resolves its asset root as `BEVY_ASSET_ROOT`, else `CARGO_MANIFEST_DIR`,
else the directory holding the executable. Only the middle one is wrong for us:
under `cargo run -p tesserae` it resolves to `crates/tesserae/`, which would
force `assets/` to live inside a BSL-licensed crate directory.

- `.cargo/config.toml` sets `BEVY_ASSET_ROOT` to the workspace root for every
  cargo-invoked run, which is the dev case.
- A shipped binary has neither variable set, so it falls through to the
  executable's directory with no configuration at all — provided the release
  archive carries `assets/` beside the binary, which it did not.

With the directory existing, `bevy/file_watcher` joins `bevy/embedded_watcher`
in the `dev` feature and the two startup warnings go away.

## Done when

`cargo run -p tesserae --features dev` starts with no asset warnings and a file
watcher on the repo-root `assets/`, and the release archive contains the binary
and `assets/` side by side.

## Notes

The directory holds a README and nothing else: placeholder textures arrive with
texturing in step 9. The README is not filler — `assets/` is **CC0, not BSL**,
by the same reasoning ADR-0004 applies to `games/demo`, and a directory that
ships inside a BSL archive under a different licence has to say so. The full
CC0 text lands with the first real asset rather than being transcribed now;
DESIGN.md's licence section is explicit about not copying licence text from
rendered web pages.

Known consequence, not solved here: `cargo install tesserae` from crates.io
would not get `assets/`, because they sit outside the crate directory. DESIGN.md
lists crates.io as a distribution channel, so this needs an answer before the
first release — probably `include`ing them in the crate, or treating crates.io
as a source-only channel. Not step 2's problem.

Look packs do not live here. They are Mods, loaded from a user directory
through the WIT contract in step 7, and ship their own WESL. `assets/` is the
engine's own default content only.

## Comments

## Comments

Mouse look turned out to be undiscoverable: it is gated on a click to grab the
cursor and nothing on screen said so, which reads as the mouse not working.
A one-line hint now sits at the bottom-left while the cursor is free and
disappears once it is grabbed. ASCII only — Bevy's embedded default font is a
FiraMono subset and renders a middle dot as tofu.

Recorded as [ADR-0005](../../../docs/adr/0005-assets-live-at-the-repository-root.md).
The reversibility cost is not the directory choice but the coupling: the
location, `BEVY_ASSET_ROOT` in `.cargo/config.toml` and the release archive's
layout have to move together, and getting it wrong yields a build that works in
development and finds nothing when shipped.
