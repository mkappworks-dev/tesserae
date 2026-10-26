# 06: Built-in solids go and ids 1 to 6 are retired

Status: ready-for-agent
Blocked by: 02, 04, 05

Spec: [decisions 1, 2, 4, 6, 8 and 9](../spec.md).

## What to build

The contract step. The only built-in Material is air.

- Ids 1 to 7 are retired: the Material table keeps a stand-in slot for each,
  the Registry a nameless slot, and a Mod's Materials still start at 8. The
  six constants, their rows and their `core:` names are deleted, not aliased.
  The single retired constant becomes the first Mod id and a retired range.
- The engine's default World parameters have no ground. A 0.2 or 0.1 Game's
  World comes up with none and the load logs a warning saying so.
- With no ground and no lakes the World is empty air, and the Server logs once
  that no Game named a ground; in a `dev` build without the built demo the
  line says to run the guest build script.
- The render palette loses its built-in colours. Air's sentinel and the flame
  stay.
- The 0.1 `builder` and `echo` guests are not rebuilt. Their tests assert what
  each returned before validation and that id 1 is refused as an unknown
  Material.
- ADR-0016 records this and amends ADR-0009 and ADR-0015. CONTEXT.md,
  architecture.md, DESIGN.md (a step 8.2 entry), README.md and CONTRIBUTING.md
  stop naming built-in solids.

## Acceptance criteria

- [ ] `core:stone`, `core:dirt`, `core:grass`, `core:lantern`, `core:glass`
      and `core:leaves` are unknown on every contract line.
- [ ] The Registry lists only `core:air` below 8, and every 0.1 fixture's Mod
      ids still start at 8.
- [ ] A player Edit or a Command naming ids 1 to 7 is refused as an unknown
      Material, and such an id already in a Chunk draws as the stand-in.
- [ ] A World with no Game is all air, the player can fly in it, and the log
      explains it.
- [ ] A 0.2 Game with World parameters loads with no ground and a warning.
- [ ] Nothing in the workspace names a deleted constant.
- [ ] ADR-0016 is written, and the docs no longer call any solid built in.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
