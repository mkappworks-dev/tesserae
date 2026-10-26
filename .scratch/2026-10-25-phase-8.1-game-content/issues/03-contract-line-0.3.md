# 03: Contract line 0.3, with 0.2 and 0.1 carried by adapters

Status: done
Blocked by: 02

Spec: [decisions 6 and 7](../spec.md).

## What to build

A new frozen-once-shipped `tesserae:mods@0.3.0` line, which the host speaks
natively. Two things change from 0.2:
- The standing-fluid record gains the lake fluid's full name. It is resolved
  after every Declaration's behaviour rows and before any `start`, with
  unknown, solid or rowless names failing the load.
- The player Edit kind gains an ignition and a put-out, each carrying the fuel's
  Material id.

Nothing else changes from 0.2. A new one-step adapter carries 0.2 to 0.3. A 0.1
component goes through the existing 0.1 adapter and then the 0.2 one. Going up,
a 0.2 or 0.1 Game's standing-fluid record, which names no fluid, is dropped
with a logged warning. Going down, ignitions and put-outs are left out of the
Edit history. The `wit/0.1` and `wit/0.2` directories are not touched.

No player action produces the new Edit kinds yet; ticket 05 does. This ticket
proves the projection with Edit histories built directly at the load seam.

## Acceptance criteria

- [x] A 0.3 Game that names a declared fluid for its lakes generates them. An
      unknown, solid or rowless name fails the load with a message naming the
      field.
- [x] A 0.2 Game that sets standing water loads, logs a warning that its World
      is dry, and generates no lakes.
- [x] Every 0.1 fixture passes unchanged, its id assertions included.
- [x] A 0.3 Mod sees ignition and put-out Edit kinds with the fuel's Material.
      0.2 and 0.1 Mods never see them, and see every other Edit as before.
- [x] A Manifest `api` that disagrees with the component's export still fails
      the load, for all three lines.
- [x] The phase 7 drain cap and the Wasmtime, WASI and Cranelift pins are
      unchanged.
- [x] ADR-0012 states that 0.3 is native, why two closed types grew, and the
      adapter chain.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- `wit/0.3` is `wit/0.2` with the package bumped, the "Closed in" comments
  moved to 0.3, `water-params` gaining `material: string` (a full name) and
  `edit-kind` gaining `ignited(u16)` and `put-out(u16)`. `wit/0.1` and
  `wit/0.2` are untouched. `bindings.rs` generates `v03` as native, and
  `SUPPORTED_API` is `0.3`, `0.2`, `0.1`.
- `adapt.rs` keeps `Line` (now with `V03`); `adapt/v01.rs` carries 0.1 to
  0.2 only and `adapt/v02.rs` carries 0.2 to 0.3. Declarations, Commands,
  `start`'s ids and World info, `default-world-params` and log levels chain
  through both. The 0.1 Edit projection still comes from the host's own
  history, because no later line's Edit kind carries the packed Voxel; it now
  excludes ignitions and put-outs by kind, not by luck of an unchanged Voxel.
- The lake resolves inside `declare::resolve`, after every fluid, reaction
  and fire row, which now returns the Game's lake. `apply` hands back the
  World parameters with no lake and checks the lake's level; `resolve` looks
  the name up and runs `validate_lake`, failing with the new
  `ModError::Lake`: "mod `game`: world: water.material `nobody:brine` names
  no registered Material" (or "is solid, not a fluid", "has no fluid row").
  The Server puts the returned lake into the Game's parameters; its own
  `validate_lake` call went, since `resolve` makes it. `bridged_default` is
  unchanged for ticket 09.
- Decision the spec leaves open: where the dry-World warning goes. The 0.2
  adapter writes it to the Mod's own log at `declare`, and the Server's load
  path now prints whatever a Mod logged during `declare` to stderr beside
  "Loaded Mod", because nothing else forwards a Mod's log. The runtime still
  forwards none of it, as before.
- Native `EditKind` gains `Ignited` and `PutOut`, each with the fuel's
  Material. Nothing produces them yet (ticket 05).
- Two guests, built by `./scripts/build-guests`: `lake`, a 0.3 Game whose
  lakes are its own `lake:brine` and which logs the Edits it is shown, and
  `pond`, a 0.2 Game that asks for standing water.
- Tests: in `tesserae-mods`, a lake of another Mod's fluid, unknown, solid
  and rowless lake names naming the field, a non-finite lake level, the
  `lake` guest's lake, the `pond` guest warned dry, a 0.1 Declaration's water
  dropped through the 0.2 adapter, and one Edit history with ignitions and
  put-outs seen by `lake` (0.3), `fluids` (0.2, unchanged and none of them)
  and `echo` (0.1, unchanged). In the Server: a 0.3 Game's valleys fill with
  its fluid, a 0.2 Game with standing water opens dry, and the Manifest
  mismatch test covers every pair of the three lines it can.
- Left for later: `core:water` unknown on every line is ticket 09, and the
  0.2 `fluids` guest still names it (ticket 04). The demo stays on 0.2
  (ticket 04). The supported-api error now lists lines comma-separated.
- `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
