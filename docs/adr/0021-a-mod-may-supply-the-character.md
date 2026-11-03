# A Mod may supply the character, and nothing else of the World

Status: accepted.

[ADR-0018](0018-games-and-mods-install-side-by-side.md) keeps ground, World
parameters, movement, character and tools with the root Game. That stops a Mod
changing a World's terrain, and it also stops a figure being shared: every Game
had to carry its own skin and bones, so the demo Games would copy about 200
lines of Rust each.

A Mod may now declare `world`, but only `world.character` is read from it. Any
other part of `world`, including `movement`, `ground`, `caves`, `smoothing` and
`water`, and any numeric field that differs from the engine's own terrain
values, is refused with the Mod's id and the field's name. `view` and `tools`
remain Game-only. A data Declaration's absent fields already mean "unset", so a
character Mod is just `[world.character]` and its bones. A component would have
to build a bare world by hand, so character Mods are expected to be data.

A World has one character source: the Game's own, or one character Mod. Two
sources fail the load and name both, whichever order they load in. Silently
preferring the Game would hide a mistake, and loosening a refusal later is
additive while tightening one is not.

This is a loader rule, not a contract change. The WIT records are untouched, so
the contract stays on line 0.3: a Declaration the loader refused before now
loads, and every existing component and Declaration behaves as it did. The
compatibility policy's "a new line is for a changed shape" is not triggered.

`tesserae validate` and `package` still load the projects they are given
together, so naming two character Mods in one command reports the conflict.
Validate or package character Mods one at a time.

Rejected: a new `[character]` table or WIT field, which is a closed-record
change and needs line 0.4 for a rule the loader can enforce today.
