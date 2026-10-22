# Mods declare once and command in batches

A Mod declares content once, receives the complete Registry at `start`, then
returns Commands from `tick`. The `tesserae:mods@0.1.0` contract has two engine
imports, `log` and `default-world-params`, and no World read or Voxel mutation
import. WASI supplies the restricted system interfaces separately. This makes
[ADR-0001](0001-wasm-mod-boundary.md)'s batched boundary enforceable: the
shipped Linker never defines the benchmark-only per-Voxel import.

**Declarations finish before the World opens.** The pre-App loader registers
Materials and Shapes in dependency-first, id-tiebroken load order. That order is
the Registry's id order; all Declarations precede every `start`. Only the Game
may supply World generation and view parameters, fixed at load time. Content
Mods cannot race to change the World's terrain or reach. A Game uses the same
loader and contract as any Mod, preserving
[ADR-0004](0004-demo-game-in-repo-real-game-out.md)'s independent guest
boundary.

**A Command describes intent in metres, never access to core.** A complete batch
is validated before acceptance. Fill membership uses Voxel lower corners in
`[min,max)`; the host converts through the World's Voxel size, as required by
[ADR-0003](0003-voxel-size-is-per-world.md). The Server drains fair per-Mod
queues through `set_voxel`, preserving shaped caps, light, mesh and LOD
invalidation. The Client receives appearance and meshes, never a queue mutator,
so [ADR-0002](0002-client-server-from-day-one.md) still has one World authority.

**Mod execution is an asynchronous Job, not work on the tick.** One owned
instance moves into a worker with events and immutable tables and returns with
its validated batch, following
[ADR-0007](0007-streaming-jobs-run-on-bevy-task-pools.md). Synchronous WASI
sleep would block a worker where guest fuel and epochs cannot interrupt it. With
`call_async`, a deadline drops the future; the hour-sleep fixture proves
cancellation while another Mod progresses. Each poll enters WASI's ambient Tokio
runtime so this also works on Bevy's executor.

**Fuel bounds guest computation; a wall deadline bounds waiting.** Fuel is
chosen over epochs for a reproducible instruction allowance. Calls yield after
10 million fuel, have 200 million fuel and a 250 ms wall deadline by default; a
Manifest may lower limits only. Memory is limited to 64 MiB. Generated bindings
lift the result before the outer-list size check; measured bounded host
allocation was accepted instead of hand-maintaining a `WasmList` ABI. Size
faults are fatal; ordinary content validation rejects the whole batch and
retains the instance for the next batch report.

**Trust authorizes an exact build.** The record binds the Mod id and kind to a
SHA-256 over length-delimited component bytes and Manifest bytes. A changed
build needs another grant; compilation uses the bytes that were checked. Trust
is permission to run within the sandbox, never permission to bypass it.

## Consequences

- Registry replay needs the same names and order. Save-time tombstones and
  missing-Mod migration remain a later decision.
- Workers never hold mutable World state. One call per Mod, bounded event
  history, accumulated reports and queue backpressure limit retained work.
- Fatal faults disable only that Mod until reopen; previously queued Commands
  still drain. Startup dispatch waits for the held, lit spawn neighborhood.
- Incremental relight makes large fills slow: the measured drain budget is two
  Voxels per tick. Bulk writes must preserve the existing Edit invariants.
- A Mod cannot read the World. A future region read needs a Snapshot and a
  batched contract rather than a per-Voxel host call.
- Export lookup is semver-compatible across 0.1 patches, as the compatibility
  fixture proves. Added `@since` exports need optional lookup; closed variants
  require a breaking contract line when extended.
