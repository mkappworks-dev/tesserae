# MaterialTable, Tables, and emission off the `match`

Status: ready-for-agent

Spec decisions 1, 5 (the sweep) and 7. Everything else stands on this, and it
changes no behaviour: the point is that the table exists, carries every property
the later tickets read, and costs the terrain hot paths nothing.

## What to build

- `tesserae-core/src/material.rs`: `MaterialDef`, `Occlusion`, `RenderClass`,
  `MaterialError`, `MaterialTable` with `builtin()`, `register`, `len`,
  `contains`, and the lookups `emission`, `dimming`, `occlusion`, `render` and
  `is_opaque`, each taking a `MaterialId` or a `Voxel` as the call sites want.
  Dense rows sized to the registered count; an unknown id answers as Opaque,
  emission 0 (spec decision 1).
- The built-in ids move here from `worldgen.rs` (`STONE` 1, `DIRT` 2, `GRASS`
  3, `LANTERN` 4) and gain `GLASS` 5, `LEAVES` 6, `WATER` 7. `worldgen` and
  `testgen` re-export the first four so no caller changes.
- `Tables { shapes: ShapeTable, materials: MaterialTable }` with `builtin()`.
  The functions that read a Material take `&Tables`: `light_chunk` and
  `relight` now. `mesh_chunk` and `greedy_mesh` keep `&ShapeTable` until ticket
  03, the first to read a Material there, so this ticket adds no parameter that
  nothing reads. `collide` keeps `&ShapeTable`; it reads geometry only.
- `light::emission` is deleted. `light_chunk` and `relight` read
  `tables.materials`, and the sweep memoises on "same Voxel as the last".
  `light.rs` stops importing `worldgen::LANTERN`.
- The app: the `Shapes` resource becomes `Tables`, the streaming jobs clone an
  `Arc<Tables>`, and call sites in tests pass `Tables::builtin()`.
- `docs/adr/0008-materials-are-table-data.md`; `CONTEXT.md` gains Dimming,
  Opaque and Translucent; `DESIGN.md` gets a pointer under step 5 and the
  "Material transparency" loose end.

## Acceptance criteria

- `register` rejects: emission or dimming above `MAX_LIGHT`; `Opaque` occlusion
  with dimming under 15 or a Blend render class; a 65 536th row. Each is a test.
- The seven built-ins have the properties in the spec's table, pinned by a test,
  and ids 0 to 7 never change.
- An unregistered id answers Opaque, dimming 15, emission 0, and does not
  panic.
- Behaviour is unchanged: every existing test passes untouched except for the
  signature change, and the mesh and light for `testgen` terrain, the lantern
  scenes and the cave worlds are identical to before.
- `cargo bench -p tesserae-core --bench meshing -- --baseline before` is run and
  the PR reports every group. `light_chunk/*` is allowed to move only if the
  PR states by how much and why; `greedy_mesh/*` and `mesh_chunk/*` must not.
- `tesserae-core` still has no dependencies; `./scripts/cargo-ci` passes.

## Comments
