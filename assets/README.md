# Assets

The engine's own default and placeholder content. Loaded at runtime from this
directory, which Bevy finds through `BEVY_ASSET_ROOT` in
[`.cargo/config.toml`](../.cargo/config.toml) during development and through the
executable's own directory in a release archive.

Mostly empty for now: texturing is step 9 of the build order in
[DESIGN.md](../DESIGN.md), and the one shader step 2 needs is embedded in the
binary rather than loaded from here, as `bevy_pbr`'s own shaders are.

## Licence

**CC0-1.0, not BUSL-1.1.** Everything else in this repository is under the
Business Source License; this directory is not, for the same reason
[ADR-0004](../docs/adr/0004-demo-game-in-repo-real-game-out.md) exempts
`games/demo`. Placeholder art that a game developer cannot reuse is placeholder
art that does its job badly, and a sharealike licence would stop a studio with
proprietary art using ours as a base — see DESIGN.md's media licence row.

The full CC0 text lands here with the first real asset. DESIGN.md is explicit
about taking licence text from a canonical source rather than a rendered web
page, and there is nothing yet for it to cover.

## What does not live here

Look packs and other Mods. They are loaded from a user directory through the WIT
contract, ship their own WESL, and are not ours to licence.
