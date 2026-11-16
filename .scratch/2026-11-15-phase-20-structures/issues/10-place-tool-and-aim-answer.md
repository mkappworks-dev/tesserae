# 10: The place Tool and its aim answer

Status: done

Blocked by: 08

Spec: [Phase 20](../spec.md).

## What to build

The Tool enum gains `place`. Its Use applies nothing, as ADR-0026
requires; the Server answers an aim with a state for each cell of the
chosen Template, so the Client draws a ghost without reading World storage. A
Game declares the Tool by name and kind.

## Acceptance criteria

- [x] A Game can declare a `place` Tool and the Catalogue lists it.
- [x] The aim answer gives each cell's state for every quarter turn and the
      Client never reads storage.
- [x] A stale aim is dropped as with Measure.
- [x] Use applies nothing and enters no history.

## Comments

For 11 (the Client):

- `place` is `tesserae_mods::ToolKind::Place` (WIT `tool-kind.place`, TOML
  `kind = "place"`). The Catalogue lists it in `tools` like `measure`.
- The Template list: `ContentCatalogue::templates`, a
  `Vec<CatalogueTemplate { name, size_m }>` in load order (the order of
  `Tables::templates`). `name` is the full `<mod id>:<name>` a
  `StructureIntent` names; `size_m` is x, y, z before any turn (a quarter
  turn swaps x and z). No kind or note is declared, so the mockup's groups
  and notes have nothing to come from yet.
- Use with the `place` Tool answers `UseOutcome(Ok(UseResult::TemplateList))`
  in Creative (open the list) and `Err(UseRefusal::CreativeOnly)` in Survival.
  It needs no target and edits nothing. `tool::aim_use` now returns
  `Option<UseAim>`; the Place Tool's is `None`, so `AimTarget::use_aim` is
  `None` while it is held. The Client's `use_tool` still sends Use on Remove
  as well as Use; 11 decides what Remove and Place mean with this Tool.
- The ghost: send `edit::StructureAim { seq, intent: StructureIntent }`
  (registered with `add_session_message`, like `LayoutAim`). The Server keeps
  the newest by `seq` (an older one is dropped, as `AimReport`'s are) and
  answers every tick with `edit::StructureAnswer { seq, ghost }`, where
  `ghost: Result<StructureGhost, RejectReason>`: `Err` is `CreativeOnly`,
  `UnknownTemplate`, or why no anchor (`OutOfReach`, `Blocked`, `NotLoaded`,
  ...). `StructureGhost { anchor, corner, size, cells }`: `corner` and
  `anchor` in Voxels, `size` in Voxels after the turn, `cells:
  Arc<[Option<StructureCell>]>` one per Voxel of the box, x fastest, then z,
  then y; `None` is Keep. `written()` yields `(at, StructureCell)` per
  written Voxel and `refused()` the first reason the placement would fail.
  Multiply `at` by the Voxel size for metres.
- `StructureCell`: `Places` (free), `ReplacesFluid` (new, for the mockup's
  "Replaces water"), `Replaces` (a solid), `Carves` (air over a solid),
  `Unchanged`, `Refused(reason)`; `Refused(NotLoaded | Busy)` waits rather
  than refuses, up to the bound. The Chunk shaped-unit cap is a whole-placement
  check and is not in the ghost.
- `StructureCell` and `StructureGhost` are re-exported from `edit` under
  `#[expect(unused_imports)]`, and `StructureGhost`'s methods under
  `expect(dead_code)` outside tests: drop those once the Client reads them.
- The Server judges the cells once and reuses them while the Template, turn,
  anchor, the Player's body where it touches the box, the box's Chunks
  (contents, load, light) and Containers stay as they were, so a 0.25 m
  Template is not judged every tick.

Not done here: no Game declares a `place` Tool yet (17, or 11 to try it in
game).
