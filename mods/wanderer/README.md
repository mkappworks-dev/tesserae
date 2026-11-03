# wanderer

A traveller in a rust-coloured coat: Minecraft-style proportions, 1.8 m tall. A
character Mod: it declares no Materials, only the figure a Game draws for the
Player.

| Declares  | Name                                                                                                                                      |
| --------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Character | six bones (body, head, two arms, two legs), a 64 by 64 `skin.png`, `pixel-m` 0.05625, and idle, walk, sprint, airborne and crouch motions |

A World has one character source: the Game's own, or one character Mod. Two
sources fail the load, naming both. Only `world.character` is read from a Mod;
any other `world` field is refused.

**Depends on:** nothing.

**Used by:** {USED}.

## Licence and artwork

MIT, in `LICENSE`. `skin.png` is original: `scripts/generate-mod-art` paints it
from code, so it has no third-party source. Edit the script and re-run it rather
than the PNG.

## Use it

```sh
tesserae validate mods/wanderer --mods mods
tesserae package mods/wanderer --out dist --mods mods
```

A Game lists it in `depends`.
