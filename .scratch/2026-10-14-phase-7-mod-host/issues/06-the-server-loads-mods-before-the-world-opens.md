# The Server loads Mods before the World opens

Status: done
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

## Comments

- Loading completes before App construction through the existing ModHost and
  bounded ModInstance calls. All/None Trust modes share a callback boundary
  for the later terminal reader. CLI overrides the environment and platform
  data-directory default; failures name the Mod and return before a window.
- Declarations are all registered before start receives the complete Registry.
  The fixture checks World information and content from a later Mod. Loaded
  tables become an immutable Arc, generation uses loaded parameters with dev
  overrides, and reach feeds StreamingSettings.
- WASI instances are Send but not Sync. LoadedMods is a temporary non-send App
  value consumed during plugin construction; the retained instances live in a
  mutex for later ownership transfer to Jobs. No tick dispatch is added here.
- MaterialPalette is plain data. All three Client terrain uniforms consume its
  sRGBA rows, retain magenta for absent appearance, and hold 64 colours. No
  Client file imports tesserae-mods.
- The optional dev fence follows content Shapes rather than assuming id 4.
  Gallery and hotbar use its registered id; a Game or a full Shape table
  suppresses it. Wasmtime/WASI 49.0.4 and Cranelift 0.136.4 are retained.
- Verification: cargo fmt --all, npm run fmt and full ./scripts/cargo-ci
  passed, including default-feature compilation, benchmark compilation,
  boundaries and cargo-deny (advisories, licenses, bans and sources). Evidence:
  target/phase7-runs/t06-ci-2.log. Workspace tests passed: app 222, core 212,
  mods 57, plus 68 doctests (three previously ignored).
- The full dev app suite passed all 250 tests, including an isolated process
  with TESSERAE_CAVES=0 and loaded parameters, and actual fence-id selection:
  target/phase7-runs/t06-dev-tests.log. The palette Startup test verifies all
  three terrain uniforms, including a missing appearance row.
- The real binary returned exit 1 for incompatible API, untrusted component
  and compilation failure, each naming the Mod before App construction:
  target/phase7-runs/t06-prewindow.log. No windowed capture was needed.
- Reviewed the final diff against every listed Test and Acceptance criterion.
  ContentTables has no post-startup writer; no tickets 07-10 are implemented.
  Interactive Trust and tick dispatch remain intentionally deferred. No
  unresolved ticket 06 acceptance criteria remain.
