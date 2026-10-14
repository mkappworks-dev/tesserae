# The Server loads Mods before the World opens

Status: ready-for-agent
Blocked by: 02, 03

Spec decisions 3, 5 and 6, and the `tesserae` half of 12. No Mod code runs after
load yet.

## What to build

- `main` loads before building the `App`: resolve the mods directory (`--mods`,
  `TESSERAE_MODS_DIR`, else the default), `discover`, `plan`, check Trust with a
  mode that is `All` or `None` for now (ticket 08 adds the prompt), compile,
  instantiate, `declare`, `apply`, `start`. The result is a plain `LoadedMods {
  tables: Tables, registry: Registry, world: Option<WorldgenParams>, view:
  Option<ViewParams>, appearance: Vec<(MaterialId, [u8; 4])>, mods:
  Vec<ModInstance> }` inserted into the `App` before `server::plugin`. No
  directory, or no Mods, is the engine's defaults exactly as today.
- `ContentTables` is built from `LoadedMods.tables` (still `Arc<Tables>`).
  `worldgen_params()` returns the loaded parameters when there are any, then
  applies the `dev` overrides on top. `StreamingSettings` takes the loaded
  `view_distance_m` and `lod_distance_m`.
- Under `dev`, the demo fence is registered only when no Game is loaded.
- `MaterialPalette`: the Server sends a plain-data message at load; the Client's
  chunk material builds its uniform from it. `PALETTE_LEN` 16 to 64 in
  `chunk_material.rs` and the WESL constant it injects. A Material with no
  appearance row stays magenta. No `client/` file imports `tesserae-mods`.
- A load error prints the Mod and the reason and exits non-zero before a window
  opens.

## Tests

- No mods directory: tables, parameters and settings equal today's, asserted
  against `Tables::builtin()` and `WorldgenParams::default()`.
- A test Mod (a `tests/guests/` fixture) declaring one Material, one Shape, a
  world with no caves and a reach of 160/3000: the Server's `ContentTables`
  holds them at ids 8 and 4, `WorldGenerator` has no caves, and
  `StreamingSettings` has the reach.
- A `dev`-only test: the loaded parameters with `TESSERAE_CAVES=0` still turn
  caves off.
- The palette message carries the fixture's colour at the fixture Material's
  index, and the Client's uniform has it there.
- Existing suites unchanged and green.

## Acceptance criteria

- `./scripts/cargo-ci` passes, including the default-features `cargo check`
  (code only `dev` uses must not be dead there).
- `ContentTables` is never written after the `App` starts.
