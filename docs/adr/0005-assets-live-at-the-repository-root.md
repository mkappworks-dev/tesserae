# Assets live at the repository root and ship beside the binary

Bevy resolves its asset root as `BEVY_ASSET_ROOT`, else `CARGO_MANIFEST_DIR`,
else the directory holding the executable. For a workspace binary the middle
rule is wrong: under `cargo run -p tesserae` it resolves to `crates/tesserae`,
which would put `assets/` inside a crate directory. Assets are **CC0, not BSL**
— for the reason [ADR-0004](0004-demo-game-in-repo-real-game-out.md) exempts
`games/demo`: placeholder art a game developer cannot reuse is placeholder art
that does its job badly.

So `assets/` sits at the repository root, beside `crates/`. `.cargo/config.toml`
pins `BEVY_ASSET_ROOT` to the workspace root for every cargo-invoked run, and a
shipped binary, which has neither variable set, falls through to its own
directory and needs no configuration at all.

## Consequences

The decision lives in three places at once: the directory, the environment
variable in `.cargo/config.toml`, and the release archive's layout in
`build-and-publish.yml`. Changing any one alone produces a build that works in
development and finds nothing when shipped, or the reverse — and a release
archive is the last thing anyone tests. That coupling, not the directory choice,
is why this is an ADR.

Release archives therefore stage the binary and `assets/` under one top-level
directory rather than packing the binary flat. Extracting must not scatter two
entries into the user's working directory.

`cargo install tesserae` does not get assets, because they sit outside the crate
directory, and DESIGN.md lists crates.io as a distribution channel. Either the
crate `include`s them or crates.io becomes a source-only channel. Unresolved,
and it must be answered before the first release.

Look packs are not covered by this. They are Mods, loaded from a user directory
through the WIT contract, shipping their own WESL. `assets/` is the engine's own
default content only.

The engine's own shaders are not assets either — they are embedded in the binary
with `embedded_asset!`, as `bevy_pbr`'s are, so they are watched by
`embedded_watcher` rather than `file_watcher`. A shader that stops hot-reloading
after being moved between the two is the expected form of this mistake.

## Third-party UI fonts (2026-10-29)

The CC0 rule applies to engine-authored media. Unmodified Liberation Sans and
Liberation Mono are bundled under `assets/fonts/` to match the approved menu
reference independently of installed system fonts. They retain upstream SIL Open
Font License 1.1, copyright and author notices; they are not relicensed as CC0.
The fonts are also embedded for deterministic UI startup. Release archives
continue to ship the root assets directory and its license notices beside the
binary.
