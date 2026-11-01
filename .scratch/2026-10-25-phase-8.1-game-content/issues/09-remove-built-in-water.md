# 09: Built-in water goes and id 7 is retired

Status: done
Blocked by: 01, 02, 03, 04, 06, 08

Spec: [decisions 1, 2, 3 and 9](../spec.md).

## What to build

The contract step of the phase. Delete the built-in water Material constant,
its Material row, the built-in water fluid row and `core:water` from the
Registry's built-in names. The built-ins are ids 0 to 6.
- **Id 7 is retired:** the Material table reports it unknown, the Registry
  never lists it, and a Mod's Materials still start at 8.
- **The built-in fluid table is empty.**
- **The engine's placeholder palette** has no water entry.
- **The translucent gallery** draws its pool with the Game's lake fluid, and
  leaves the pool out when there is none.
- **The temporary lake bridge from ticket 02 goes:** a World with no Game is
  dry.

Write ADR-0015 amending ADR-0009's permanent ids. It notes what this changes in
ADR-0012 and ADR-0013, describes the approach in Tesserae's own terms and names
no other engine. Update CONTEXT.md (Fluid, Source, Water level),
architecture.md's "Mechanics are the engine's, content is the Game's" section
and its ADR list, and DESIGN.md.

## Acceptance criteria

- [x] Nothing in the workspace refers to a built-in water Material, a built-in
      fluid row or `core:water`, except where the ADR and docs say they were
      removed.
- [x] Id 7 is unknown to the Material table and absent from the Registry. A Mod
      Material registered first gets 8, and the 0.1 world-loader fixture passes
      unchanged.
- [x] A Chunk holding id 7 draws the opaque stand-in and is never treated as a
      fluid.
- [x] A World with no Game has no lakes and no fluid rows. Igniting anything is
      refused as not flammable, and the default hotbar offers no fluid and no
      fire.
- [ ] With the demo loaded, lakes, the gallery pool, scenarios and LOD water
      look as they do on main, confirmed by a screenshot pass reviewed with the
      user and then deleted. Screenshots in /tmp/tesserae-p81-t09/ await the
      user's review.
- [x] ADR-0015 exists and is linked from ADR-0009 and the architecture ADR
      list. The glossary and architecture table no longer say water is built in.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- **Id 7 in the table.** `material::RETIRED` (7). `MaterialTable::builtin()`
  still has eight dense rows; row 7 is `MaterialDef::OPAQUE`, the answer any
  unknown id gets, and `contains` is `id < len && id != RETIRED`. `len()` is
  documented as the next id, retired slot included, so `register` still hands
  out 8 first and nothing that indexes rows changes. `def(7)` draws the
  stand-in; `vet_placed`, `register_fluid` and combustion refuse 7 as unknown.
- **Id 7 in the Registry.** Its fields are now private
  `Vec<Option<String>>`; slot 7 is `None`. `names(kind)` yields `(id, name)`
  and skips it, `id_of` searches only those, `len(kind)` is the next id, and
  `replay` compares slots, so a save with a name at 7 is `BuiltinMoved`. The
  Server builds `start`'s ids and the catalogue from `names`, so no line, and
  no menu, ever lists 7. The 0.1 world-loader fixture passes unchanged.
- **Removed:** `material::WATER`, its row, `FluidDef::WATER` (the table is
  empty, `register` inlines the old `set`), `core:water`, the palette entry,
  `bridged_default` (no Game is `WorldgenParams::default()`, dry).
- **Decision, the gallery:** it ran only with no Game, because it needs the
  dev fence. Story 32 wants it with the demo, so under `dev` it now always
  runs; with a Game its fence line is left out (a Game's fence is content)
  and its pool is the World parameters' lake fluid, or no basin at all.
- **Tests added:** retired id unknown, stand-in and a Mod's first id 8
  (`material.rs`); no fluid built in (`fluid.rs`); 7 meshes as an opaque
  cube, hides its neighbour's face and reads as no fluid beside a registered
  one (`tests/meshing.rs`); built-ins 0 to 6, 7 never listed, replay
  (`tests/declare.rs`); `core:water` unknown in a 0.3 reaction and a 0.2
  fluid row through the adapter, and a Command of id 7 refused on 0.1, 0.2
  and 0.3 (`behaviour.rs`); no Game: dry parameters (`server/mod.rs`), every
  built-in solid refused as "that does not burn" (`edit.rs`), a catalogue of
  the seven built-ins with no lake, fluid or fuel (`mods.rs`); the pool is
  the lake fluid and absent without one (`gallery.rs`). The default hotbar
  for a dry catalogue was already ticket 06's test. Tests that used
  `core:water` as a second fluid declare `water:water` instead.
- **Remaining grep hits** for `core:water` are those tests asserting it is
  unknown, in `behaviour.rs`, `declare.rs` and the demo runtime test.
- **Docs:** ADR-0015, linked from ADR-0009 and architecture's list; CONTEXT
  (Water level, Fluid, Source, Waterline); architecture's mechanics table;
  DESIGN step 8 and open items; README and CONTRIBUTING say a World with no
  Game is dry; the 0.3 `declaration` comment says Materials start at 8.
- **For ticket 10:** a fluid can no longer be registered at 7, so its
  "bench water at 8 and at 7 in one binary" probe needs a pre-09 build.
- **Screenshots**, offscreen, 1.0 m, in `/tmp/tesserae-p81-t09/`, with main
  (9992e8e) built in a temporary worktree for comparison: `branch-demo-*` and
  `main-nogame-*` show lakes and the gallery pool in the same colour (the
  demo's World is flatter, so the terrain differs); `*-demo-beam` and
  `*-demo-fire-late` match; at 12 s (`*-demo-fire`) the branch's fire row is
  a step behind main's, the same state by 40 s; `branch-nogame-*` is dry with
  the gallery and no pool; `branch-*-menu` lists no `core:water`.
- `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass: tesserae
  370 (2 ignored), core 326, meshing 69, tesserae-mods 95 (3 ignored).
