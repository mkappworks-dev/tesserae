# Games and Mods are installed side by side; a World freezes its set

Games live in `games/<id>/` and Mods in `mods/<id>/`, siblings of `worlds/` and
`trust.json`; `--mods` still names the Mods directory. A root manifest is valid
only under `games/`, a non-root one only under `mods/`, and one id names one
thing across both. The scan reads `mod.toml` only: it never hashes, compiles or
runs anything, and a bad, symlinked, duplicate or misplaced folder becomes a
rejected row instead of failing the list. A legacy root under `mods/` moves once
to `games/`; Trust is keyed by id and content hash, so it survives the move.

A World records the Game and the Mods chosen at Create, dependencies included,
in its sidecar and its commit's required content, and loads exactly that set. A
missing or reversioned item is refused before any write, naming each one. There
is no "open anyway": the commit's exact build hash still decides compatibility.
Trust is asked only for the set about to open.

Ground, World parameters, movement, character and tools come from the root Game
alone; a Mod adds Materials, appearance and textures. The Engine still ships
only Air ([ADR-0016](0016-only-air-is-built-in.md)).

Rejected: loading every installed Mod into every World, which lets an unrelated
install change a save, and side-by-side versions of one id, which need a
compatibility field and a resolver this engine has no use for yet.
