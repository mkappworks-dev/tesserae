# Contract compatibility

The Mod contract is the WIT bundle `tesserae:mods@<line>.<patch>`, published
under `crates/tesserae-mods/wit/<line>/` and copied into
[`starters/rust/wit/`](../starters/rust/wit/). A Manifest's `api` field names
the **line** (`0.3`), never a patch.

## Patches and lines

- **A patch (`0.3.0` to `0.3.1`) is compatible.** It may change documentation,
  comments and clarifications of existing behaviour. A component built against
  any `0.3.x` loads on every Engine that supports line `0.3`.
- **A new line (`0.3` to `0.4`) is breaking.** Any change to a closed record or
  variant, a removed or retyped field, or a changed function signature is a new
  line. Adding a field to a record counts: records are closed, so the layout
  changes.
- The Engine keeps hosting older lines through adapters that map them to the
  current one, under their existing restrictions. A component or Declaration for
  line `0.1`, `0.2` or `0.3` keeps loading until a release announces that line's
  end. Data Declarations exist only on `0.3`.
- A component exports one line. Moving a Game to a newer line means rebuilding
  it against that line's bundle; the Engine never rewrites a component.

## Loader rules are not the contract

A change to what the loader accepts without changing a WIT record is not a new
line. [ADR-0021](adr/0021-a-mod-may-supply-the-character.md) lets a Mod supply
`world.character` on line 0.3: nothing that loaded before behaves differently.

## Amended before release

Line 0.3 gained `items` on its `declaration` while unreleased. A Declaration for
0.1 or 0.2 loads with no items.

## What is not frozen

Line `0.x` is pre-release. Later gameplay steps may add new lines, and
publishing a starter does not freeze the Tools, commands or simulation features
those steps add. The final **1.0 contract freeze belongs to roadmap step 27**
(release hardening); until then, treat any new line as possible.

Saved Worlds are separate: they require the exact build that created them
([ADR-0018](adr/0018-games-and-mods-install-side-by-side.md)), whichever line it
used.
