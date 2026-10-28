# Assets

The engine's own default and placeholder content. Loaded at runtime from this
directory, which Bevy finds through `BEVY_ASSET_ROOT` in
[`.cargo/config.toml`](../.cargo/config.toml) during development and through the
executable's own directory in a release archive.

The menu fonts are bundled here. Engine-authored media follows the CC0 rule
below; third-party fonts retain their upstream license. Engine shaders are
embedded in the binary, as `bevy_pbr`'s own shaders are.

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

## Bundled UI fonts

The files under `fonts/` are Liberation Fonts 2.1.5, bundled unchanged under SIL
Open Font License 1.1, rather than CC0. Their copyright, license and author
notices are retained beside them. The source archive is the
[upstream 2.1.5 binary release](https://github.com/liberationfonts/liberation-fonts/files/7261482/liberation-fonts-ttf-2.1.5.tar.gz).
Liberation Sans and Liberation Mono match the approved menu prototype's fonts.
