# sprite

A small, quick figure with leaf-green hair, 1.44 m tall. A character Mod: it
declares no Materials, only the figure a Game draws for the Player.

| Declares  | Name                                                                                                                                    |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Character | six bones (body, head, two arms, two legs), a 64 by 64 `skin.png`, `pixel-m` 0.045, and idle, walk, sprint, airborne and crouch motions |

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
tesserae validate mods/sprite --mods mods
tesserae package mods/sprite --out dist --mods mods
```

A Game lists it in `depends`.
