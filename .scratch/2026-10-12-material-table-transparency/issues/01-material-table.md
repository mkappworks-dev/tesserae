# MaterialTable, Tables, and emission off the `match`

Status: done

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

**Benchmarks, and why they are instruction counts.** The machine was shared with
other builds (load average 25 to 120), and criterion's wall time was not usable:
a `--baseline before` run showed untouched code (`worldgen/*`,
`shape_table_builtin`) moving by +25% to +240%, and an interleaved old/new run
had 80-270% spread inside one configuration. So the comparison is retired
instructions per call (`/usr/bin/time -l`, 400 calls less the setup, three runs
each, spread under 0.5%), on a harness built against this tree and against
`f232c4e`:

| `light_chunk` case                   | before    | after     | change |
| ------------------------------------ | --------- | --------- | ------ |
| `straddling` (terrain, sky above)    | 3 342 000 | 3 253 000 | -2.7%  |
| `straddling_settled` (both channels) | 4 843 000 | 4 735 000 | -2.2%  |

The sweep memoises on "same Voxel as the last", so it does fewer table reads
than the old code did `match`es. `mesh.rs`, `chunk.rs` and `collide.rs` are
untouched, so no `greedy_mesh/*` or `mesh_chunk/*` number can have moved.

**Latent until 02:** the table said glass dims by 0 but nothing read `dimming`,
so a glass Voxel still lit and meshed as an opaque cube. Tickets 02 and 03 were
done in the same PR and read it; see their comments.
