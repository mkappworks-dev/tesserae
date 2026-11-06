# Assets

The engine's own default and placeholder content. Loaded at runtime from this
directory, which Bevy finds through `BEVY_ASSET_ROOT` in
[`.cargo/config.toml`](../.cargo/config.toml) during development and through the
executable's own directory in a release archive.

The UI fonts are bundled here, under CC0 or the SIL Open Font License 1.1.
Engine shaders are embedded in the binary, as `bevy_pbr`'s own shaders are.

## Licence

**CC0-1.0, not BUSL-1.1**, except fonts under the SIL Open Font License 1.1,
each of which keeps its licence text beside it
([ADR-0005](../docs/adr/0005-assets-live-at-the-repository-root.md#fonts-under-the-sil-open-font-license)).
Everything else in this repository is under the Business Source License; this
directory is not, for the same reason
[ADR-0004](../docs/adr/0004-terra-game-in-repo-real-game-out.md) exempts
`games/terra`. Placeholder art that a game developer cannot reuse is placeholder
art that does its job badly, and a sharealike licence would stop a studio with
proprietary art using ours as a base — see DESIGN.md's media licence row.

The [CC0 dedication](https://creativecommons.org/publicdomain/zero/1.0/) permits
reuse, modification and redistribution, including commercial use, without an
attribution or license-notice requirement.

## What does not live here

Look packs and other Mods. They are loaded from a user directory through the WIT
contract, ship their own WESL, and are not ours to licence.

## Bundled UI fonts

Both faces are embedded in the binary for deterministic native UI startup.

| File                         | Font                                 | Licence                                            | Used for                                                   |
| ---------------------------- | ------------------------------------ | -------------------------------------------------- | ---------------------------------------------------------- |
| `fonts/InterVariable.ttf`    | Inter 4.1, variable (`wght`, `opsz`) | SIL OFL 1.1, text in `fonts/InterVariable-OFL.txt` | All interface text; Bevy's default Font asset (HUD, saves) |
| `fonts/Aileron-SemiBold.otf` | Aileron 0.102 SemiBold               | CC0                                                | The Tesserae wordmark only                                 |

`InterVariable.ttf` and its licence are unchanged from the
[official Inter 4.1 release](https://github.com/rsms/inter/releases/tag/v4.1)
(`Inter-4.1.zip`, SHA-256
`9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e`). The font
(SHA-256 `4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf`) and
`LICENSE.txt` are byte-identical to `docs/font-files/InterVariable.ttf` and
`LICENSE.txt` at the repository's `v4.1` tag. Inter declares no Reserved Font
Name.

Aileron SemiBold is from the designer's
[official 0.102 archive](https://dotcolon.net/files/fonts/aileron_0102.zip). The
[designer's page](https://dotcolon.net/fonts/aileron/) dedicates Aileron to the
public domain under CC0; the font's embedded notice reads “No Rights Reserved.”
This provenance record is informational, not a required font notice.
