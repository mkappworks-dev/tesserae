# Assets

The engine's own default and placeholder content. Loaded at runtime from this
directory, which Bevy finds through `BEVY_ASSET_ROOT` in
[`.cargo/config.toml`](../.cargo/config.toml) during development and through the
executable's own directory in a release archive.

The UI fonts are bundled here under CC0. Engine shaders are embedded in the
binary, as `bevy_pbr`'s own shaders are.

## Licence

**CC0-1.0, not BUSL-1.1.** Everything else in this repository is under the
Business Source License; this directory is not, for the same reason
[ADR-0004](../docs/adr/0004-demo-game-in-repo-real-game-out.md) exempts
`games/demo`. Placeholder art that a game developer cannot reuse is placeholder
art that does its job badly, and a sharealike licence would stop a studio with
proprietary art using ours as a base — see DESIGN.md's media licence row.

The [CC0 dedication](https://creativecommons.org/publicdomain/zero/1.0/) permits
reuse, modification and redistribution, including commercial use, without an
attribution or license-notice requirement.

## What does not live here

Look packs and other Mods. They are loaded from a user directory through the WIT
contract, ship their own WESL, and are not ours to licence.

## Bundled UI fonts

The unchanged Aileron Regular, SemiBold and Bold faces are from the designer's
[official 0.102 archive](https://dotcolon.net/files/fonts/aileron_0102.zip). The
[designer's page](https://dotcolon.net/fonts/aileron/) dedicates Aileron to the
public domain under CC0; each font's embedded notice reads “No Rights Reserved.”
This provenance record is informational, not a required font notice. The same
faces are embedded for deterministic native UI startup and used by the menu
reference, including key labels and numeric values. Aileron Regular also
replaces Bevy's default Font asset for HUD text and save dialogs.
