# Mods declare once and command in batches

A Mod declares content once, receives the complete Registry at `start`, then
returns Commands from `tick`. Both contract lines, `tesserae:mods@0.1.0` and
`tesserae:mods@0.2.0`, have the same two engine imports, `log` and
`default-world-params`, and no World read or Voxel mutation import. WASI
supplies the restricted system interfaces separately. This makes
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
queues through the one World change a player's Edit takes, `set_occupants`,
preserving shaped caps, light, mesh and LOD invalidation. The Client receives
appearance and meshes, never a queue mutator, so
[ADR-0002](0002-client-server-from-day-one.md) still has one World authority.

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

**Fluids and fire are a new contract line, not a 0.1 patch.** `0.2` adds
`fluids`, `reactions` and `combustion` to the Declaration, the `place-sources`,
`remove-sources`, `ignite` and `extinguish` Commands, and a typed player Edit
history (`solid`, `source-placed`, `source-removed`). Each of those grows a
closed record or variant, which 0.1's export lookup cannot see, so 0.1 stays
closed and frozen in `wit/0.1/` and `wit/0.2/` is the line the host speaks
natively. Behaviour rows name Materials by full name and are resolved after the
last Declaration and before any `start`, so a row may name another Mod's
Material, and an unknown name, a non-finite value, a value out of range or two
rows that could match one contact fail the load. On 0.2 a fluid placed through
`place` or `fill` is refused: fluid goes in only as a source, through
`place-sources`. The new Commands are region Commands under the same caps and
lazy iteration as `fill`, and reach the same World change; they ask for a state
and never call into the simulation.

**0.1 components keep loading through an explicit adapter.** The component's
`guest@` export says its line, and a Manifest `api` that disagrees fails the
load. The adapter carries a 0.1 Declaration in with no behaviour rows, 0.1
Commands in as their native equivalents, and the native tick context out as 0.1
saw it: a 0.1 `place` or `fill` of a fluid Material is a whole-Voxel source, as
it always was, and a 0.1 Mod sees each player Edit as its packed-Voxel
projection, so an Edit that leaves the packed Voxel unchanged, waterlogging a
stair, is not one it sees. One Linker holds both lines' imports. Every existing
0.1 fixture passes unchanged against it.

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
- Export lookup is semver-compatible across patches within a line, as the
  compatibility fixture proves. Added `@since` exports need optional lookup;
  closed variants require a breaking contract line when extended, as 0.2 was.
- The adapter is permanent cost while 0.1 is supported: every line the host
  accepts is a projection to keep true as the native line grows. Dropping 0.1 is
  a separate decision.
- A 0.2 Command can only ask for sources and fires the Server's own rules allow.
  A Mod cannot set flow, reach, exposure or ignition progress directly.
