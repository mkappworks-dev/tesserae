# 03: Contract line 0.3, with 0.2 and 0.1 carried by adapters

Status: ready-for-agent
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

- [ ] A 0.3 Game that names a declared fluid for its lakes generates them. An
      unknown, solid or rowless name fails the load with a message naming the
      field.
- [ ] A 0.2 Game that sets standing water loads, logs a warning that its World
      is dry, and generates no lakes.
- [ ] Every 0.1 fixture passes unchanged, its id assertions included.
- [ ] A 0.3 Mod sees ignition and put-out Edit kinds with the fuel's Material.
      0.2 and 0.1 Mods never see them, and see every other Edit as before.
- [ ] A Manifest `api` that disagrees with the component's export still fails
      the load, for all three lines.
- [ ] The phase 7 drain cap and the Wasmtime, WASI and Cranelift pins are
      unchanged.
- [ ] ADR-0012 states that 0.3 is native, why two closed types grew, and the
      adapter chain.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
