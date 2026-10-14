# Mod host: tesserae-mods

Build-order step 7. Branch `feat/phase-7-mod-host`.

`tesserae-mods` loads a Mod, a WASM component, against a versioned WIT contract
and lets it do two things: declare content once at load (Shapes, Materials,
World generation parameters, the reach), and answer a tick with a batch of
Commands the Server applies through the Edit path that already exists. It is a
wasmtime host with no Bevy in it, a Server-side plugin that runs Mod code as
Jobs, and a demo Game that proves a Game can be built from the contract alone.

Seven things were decided before this spec and are followed, not argued again:

- ADR-0001: the boundary is batched, WASM is never per Voxel, and generation,
  meshing, light and fluids never run in WASM.
- ADR-0002: only the Server mutates World state. A Mod is not a Client, but it
  is not the Server either: it asks, and the Server applies.
- ADR-0003: the Mod API is in metres.
- ADR-0004: `games/demo` is in-repo, outside the workspace, licensed MIT or CC0,
  and may use only what the WIT contract exposes.
- ADR-0006 and ADR-0009: the Shape and Material tables are filled by `register`
  at load; ids are save data; a Mod's ids start at 4 and 8 in registration
  order.
- ADR-0007: Jobs hold Arcs, return owned values, and land on the tick. The
  tables are immutable once the World is open.
- Phase 6 decision 10: a Mod sets the full-detail radius and the reach once at
  load, in the same `Copy` data `WorldgenParams` is.

## Vocabulary this step adds

To go into `CONTEXT.md` in the same PR.

- **Manifest** - the file beside a Mod's component that names it: id, version,
  which contract version it was built against, whether it is a Game.
- **Declaration** - what a Mod returns from `declare` at load: the Materials and
  Shapes it adds, and, from a Game only, the World parameters and reach. Plain
  data, returned once.
- **Registry** - the ordered list of names a World's `MaterialId`s and
  `ShapeId`s were handed out in. Save data once there is a save.
- **Command** - one thing a Mod asks the Server to do to Voxels: fill a region,
  or place a list of Voxels. Addressed in metres.
- **Command batch** - what one call of a Mod's `tick` returns. The only way a
  Mod acts on the World after load.
- **Trust** - the user's recorded yes to one exact build of one Mod.

## What the design rests on

Read from sources before deciding, because several decisions below only make
sense once these are known.

- **wasmtime is 49.0.2** (docs.rs, 2026-10-02). Its `rust-version` is 1.96, and
  the workspace is 1.97. Default features include `component-model`, `async`,
  `cranelift`, `pooling-allocator` and `gc`. A probe crate with
  `default-features = false` and `runtime, cranelift, component-model,
  pooling-allocator, parallel-compilation, std`, plus `wasmtime-wasi`, resolves
  289 crates with no `bevy` or `wgpu` among them, and `cargo deny check licenses
  advisories bans sources` against this repo's `deny.toml` finds nothing wrong
  with any of them. `wasmtime-wasi` brings `tokio`.
- **The component Linker matches names across compatible versions.** A host that
  defines `pkg@0.2.1` satisfies a guest import of `pkg@0.2.0` and the reverse,
  on the stated assumption that an interface is never changed once defined
  (docs.rs, `component::Linker`). The page does not say what counts as
  compatible, and says nothing about export lookup.
- **WIT versioning** (`WebAssembly/component-model`, `design/mvp/WIT.md`): a
  package carries `@x.y.z`; `@since(version = ...)`, `@unstable(feature = ...)`
  and `@deprecated(version = ...)` gate items; adding a case to a `variant` is a
  breaking change; and under SemVer 0.x a change in the minor position is a
  major one. `async func`, `stream` and `future` are in the grammar, and
  wasmtime marks its async component-model support "very incomplete"
  (`Config::wasm_component_model_async`).
- **A trap poisons the Store.** `may_enter` is false for a component Store once
  it has trapped (`wasmtime-49.0.2/src/runtime/component/store.rs`), so a Mod
  that traps is dropped and not retried. Fuel is exact and costs more than
  epochs, which are about 10% and not deterministic (docs.wasmtime.dev,
  "Interrupting Wasm"). A new Store has 0 fuel and traps at once until
  `set_fuel`.
- **Limits exist and default wide.** `StoreLimitsBuilder` has `memory_size`,
  `table_elements`, `instances`, `tables`, `memories` and `trap_on_grow_failure`
  (`wasmtime-49.0.2/src/runtime/limits.rs`); the default is 10 000 instances,
  memories and tables. `max_wasm_stack` defaults to 512 KiB.
  `memory_reservation` defaults to 4 GiB of address space per memory on 64-bit.
  `wasm_relaxed_simd` defaults on and its results differ across hosts.
- **WASI p2 is opt-in per capability.** `wasmtime_wasi::p2::add_to_linker_async`
  exists; a `WasiCtx` inherits nothing unless a builder method says so
  (`inherit_*`, `preopened_dir`, `inherit_network`); stdin is closed;
  `ip-name-lookup` is denied; `secure_random` and `insecure_random_seed` can be
  replaced (`wasmtime-wasi-49.0.2/src/ctx.rs`). Blocking WASI calls (a sleep, a
  poll) block the host thread when driven synchronously, which is why decision 9
  drives them asynchronously.
- **`wasm32-wasip2` is Tier 2** and produces a component directly, with
  `-Cpanic=abort` (rustc platform support). It is **not installed on this
  machine**: `rustup target list --installed` lists only `aarch64-apple-darwin`.
  Building any guest needs `rustup target add wasm32-wasip2`, a download, which
  ticket 01 asks the user to approve.
- **There is no Registry in the code.** `ShapeTable::register` and
  `MaterialTable::register` hand out ids in order and are the only record. The
  Server's `ContentTables` is `Arc<Tables>` built by `Default` (plus a dev-only
  fence registered as a Mod will), `worldgen_params()` is a function with `dev`
  environment overrides, and `StreamingSettings` holds `view_distance_m` and
  `lod_distance_m` as plain fields.
- **A Material has no colour.** ADR-0009 puts appearance in Look packs, and the
  chunk shader tints by `palette[material]`, a `[Vec4; 16]` uniform whose
  unfilled entries are magenta and whose index clamps to the last
  (`render/chunk_material.rs`, `PALETTE_LEN`). A Mod's first Material is id 8
  and would draw magenta. Decision 6.
- **Edits are one path.** `edit::set_voxel` is the only writer, shared by player
  intents and the `dev` builders; it enforces the shaped cap, relights, marks
  meshes stale and, through `voxels_changed`, invalidates the level-1 cover. The
  tick applies at most `EDITS_PER_TICK` (16) of them, from a `PendingEdits`
  queue only the Server can push to.

## Decisions

### 1. The contract: package `tesserae:mods`, one world

`crates/tesserae-mods/wit/` holds the contract. It is the artifact ADR-0004's
split is a test of: the demo and every Game outside the repo build against these
files and nothing else.

```wit
package tesserae:mods@0.1.0;

interface types {
    record vec3 { x: f32, y: f32, z: f32 }       // metres
    record box-m { min: vec3, max: vec3 }        // metres, [min, max)
    record voxel { material: u16, shape: u8, orientation: u8 }
    // ... node-box, collision-box, connect-def, arm: ShapeDef's own fields
}

interface content {
    use types.{...};
    record material-def { name: string, emission: u8, dimming: u8,
                          occlusion: occlusion, render: render-class,
                          solid: bool }
    record shape-def { name: string, boxes: list<node-box>,
                       collision: option<list<collision-box>>,
                       connect: option<connect-def> }
    record appearance { material: string, color: srgba }
    record world-params { ... }                  // WorldgenParams, in metres
    record view-params { view-distance-m: f32, lod-distance-m: f32 }
    record declaration { materials: list<material-def>,
                         shapes: list<shape-def>,
                         appearance: list<appearance>,
                         world: option<world-params>,
                         view: option<view-params> }
}

interface commands {
    use types.{...};
    record placement { at: vec3, voxel: voxel }
    variant command { fill(fill), place(list<placement>) }
    record fill { region: box-m, voxel: voxel }
}

interface host {                                 // the whole import surface
    log: func(level: log-level, message: string);
    default-world-params: func() -> world-params;
}

interface guest {                                // what a Mod exports
    use content.{declaration}; use commands.{command};
    declare: func() -> declaration;
    start: func(world: world-info, ids: registered);
    tick: func(ctx: tick-context) -> list<command>;
}

world mod { import host; export guest; }
```

The sketch fixes the shape, not every field; ticket 01 writes the real file and
a test that the Rust types it generates round-trip to `tesserae-core`'s own
`ShapeDef` and `MaterialDef`.

The import surface is **two functions**, neither of which touches a Voxel. A
test lists the imports of a built component and fails on anything outside
`host`, the WASI interfaces decision 10 links, and nothing else: that is
ADR-0001 as a test and not a convention. There is no `get-voxel`, no
`set-voxel`, no region read. A Mod that needs the World's contents is a later
decision (Known loose ends), because it needs a Snapshot to read from.

### 2. Versioning

The package is `tesserae:mods@0.1.0` and every item is stable from it. Under 0.x
a minor bump is a breaking change, so:

- **Additive** (no bump of the line): a new function, interface, type or world
  import/export, marked `@since(version = 0.1.1)`. Nothing existing changes.
  This includes not adding a case to a `variant` or `enum`, which WIT.md calls
  breaking: `command` and `occlusion` are closed in 0.1, and growing one is a
  `0.2.0`.
- **Breaking** (`0.2.0`): anything else. The host supports the current line and
  the one before it, each with its own bindgen module and Linker, and rejects an
  older Mod with a message that names both versions. One line back is a
  commitment this step can afford because there is exactly one line.
- The Manifest says `api = "0.1"`. The host reads it **before** compiling, so an
  incompatible Mod fails in milliseconds with the right sentence and not with a
  linker error.

What is not known and ticket 01 settles with a test before anything relies on
it: whether a component built against `0.1.0` instantiates on a host that
defines `0.1.1` for its **exports** as well as its imports. The Linker page
covers imports only. If export lookup is exact, the host looks up `guest@0.1.0`
and falls back to the Manifest's declared patch.

Anything physical in the contract is in metres (ADR-0003): `vec3`, `box-m`,
`world-params`, `view-params`. A Voxel count appears only as a cost cap
(decision 8), never as a size. `world-info` hands the Mod `voxel_size_m` and the
seed once, so a Mod that wants to align to the lattice can, and none assumes a
Voxel is a metre.

### 3. Loading, and the Manifest

A mods directory holds one directory per Mod:

```
<mods>/<id>/mod.toml        manifest
<mods>/<id>/<wasm>          the component
```

```toml
id = "demo"                 # [a-z][a-z0-9_]{0,31}; the namespace of its names
version = "0.1.0"           # the Mod's own, semver
api = "0.1"                 # the contract line it was built against
root = true                 # a Game; at most one
wasm = "demo.wasm"
depends = ["other"]         # ids; loaded first
```

Sequence, in order, and each step fails the whole load with a message naming the
Mod: discover, parse Manifests, check `api` and `root` count and `depends`,
check Trust (decision 11), compile, `instantiate_pre`, instantiate, `declare`,
validate, register, `start`. Order is dependencies first, then by id, so it is
the same on every machine; **registration order is load order**, which is what
makes a Registry reproducible.

The Manifest is TOML parsed with `toml` and `serde`; both are in the licence
allowlist and `tesserae-mods` is the only crate that gains them. It is a file
beside the component and not an export of it so the host can refuse a Mod
without running a byte of it. A directory with no Manifest is skipped with a
warning, not an error, so a stray folder in the user's mods directory cannot
stop the game.

The default mods directory is the platform's user data directory plus
`tesserae/mods` (`dirs`), overridden by `--mods <dir>` or `TESSERAE_MODS_DIR`.
ADR-0005 already put Mods in a user directory, outside `assets/`.

Loading is **synchronous and before the Bevy `App` is built**, in `main`. The
Server needs the tables, the generator's parameters and the reach at `Startup`,
and all of them are immutable after (ADR-0007), so there is no Startup ordering
edge to maintain and no half-loaded World. Compiling a 1 MB component is the
cost accepted; precompiled caches (`Engine::precompile_component`) are a loose
end.

### 4. Declarations become table rows, and a Registry records the order

`declare` returns data; the host turns it into `tesserae-core` types and calls
`MaterialTable::register` and `ShapeTable::register` once per row, in
declaration order, Mod by Mod in load order. Those calls already validate
(`MaterialError`, `ShapeError`); a failure is a load error naming the Mod and
the row. Nothing else reaches `tesserae-core`: ADR-0001's second invariant.

Names are `<mod id>:<name>` with the Mod's local part `[a-z][a-z0-9_]{0,31}`,
which the host prefixes; a Mod cannot name into another's namespace. Built-ins
are `core:air` to `core:water` and `core:cube`, `core:slab`, `core:stair`,
`core:post` at their permanent ids. A duplicate name is a load error.

`Registry { materials: Vec<String>, shapes: Vec<String> }` is the ordered list;
a name's id is its index. It lives in `tesserae-mods`, is plain data, and has
`names()` and `replay(saved)`. `replay` loads in the saved order and fails with
`RegistryMismatch` if a saved name is not provided by this load. **Step 7 writes
no file**, because there is no save, so `replay` is tested and not yet used. The
case a save step must decide is a missing Mod: ids are dense, so keeping a later
Mod's ids stable needs a filler row for the hole, and ADR-0006's "unknown id is
invisible" only covers ids past the end. That is a loose end, named here so the
save step does not discover it.

A Mod never hardcodes an id. `start(world, ids)` passes it the name-to-id map of
everything registered, built-ins included, and its Commands carry those ids. The
host re-checks them anyway (decision 8), because the table is the registry.

Limits at registration, constants in `ModLimits`: at most 56 Materials in total
across all Mods (the palette, decision 6) and at most 252 Shapes in total (the
`ShapeId` space after the built-ins), at most 64 of either per Mod, names at
most 32 bytes.

### 5. World parameters, and who may set them

`WorldgenParams`, `CaveParams`, `SmoothingParams`, `WaterParams` and the two
distances are `Copy` data with no callbacks, already, so `world-params` is a
field-for-field copy and `view-params` is two numbers. Nothing here is new
mechanism. What this step decides is **who**.

- **Only the Game sets them**, the one Mod whose Manifest says `root = true`. A
  Declaration from any other Mod with `world` or `view` set is a load error, not
  ignored, so a Mod author learns it at once. This avoids inventing merge rules
  for two Mods that both want a sea level, and it matches CONTEXT.md: a Game is
  the root, content Mods extend it.
- **The record replaces; it does not patch.** `world: none` is the engine's
  default. `world: some(p)` is exactly `p`, including `caves: none` meaning no
  Caves, as `WorldgenParams` means it. A Game that wants the default with a
  taller relief calls `host.default-world-params()`, edits one field and returns
  it. That is the second import, and it exists so a patch is possible without
  the contract growing an `option<option<_>>`.
- **Validated before use**, in metres, finite, and in the ranges the generator
  assumes: `feature_size_m > 0`, `relief_m >= 0`, `soil_depth_m >= 0`, `octaves`
  in `1..=8`, a Cave's `radius_m` and `feature_size_m` positive. Ticket 03 reads
  `worldgen.rs` and `noise.rs` for the real bounds and adds
  `WorldgenParams::validate` in core (pure, no dependency); a Mod that sets a
  water level above the surface band gets the generator's own clamp, as
  `WaterParams` documents.
- **The reach**: `view_distance_m` in `32..=512` and `lod_distance_m` in
  `view_distance_m..=8192`, so the level count stays inside `MAX_LEVELS` and the
  per-level table in DESIGN.md's open items is the ceiling. These are chosen,
  not derived; ticket 03 checks the top of each range against that table and
  moves it if the numbers are wrong. They go into `StreamingSettings`. Neither
  is World save data, as phase 6 decided.

The `dev` overrides (`TESSERAE_CAVES`, `TESSERAE_SMOOTHING`,
`TESSERAE_WATER_LEVEL_M`) apply on top of the loaded parameters, so a sweep
still needs no rebuild.

### 6. Appearance is a placeholder, and it is not in the table

A Mod's Material needs a colour or it draws magenta, and ADR-0009 keeps colour
out of `MaterialDef`. So the Declaration carries `appearance`, a separate list
of `(material name, sRGBA)`, consumed by the Client's flat palette and by
nothing in `tesserae-core`. It is a stopgap with a date on it: step 9 replaces
the palette with a Look pack's texture array and this list goes with it, which
is why it is beside the Materials and not inside them.

It reaches the Client the way meshes do: the Server, which owns the tables,
sends a `MaterialPalette` message at load (plain data, no `Entity`), and the
Client builds its uniform from it. That is content and not World state, so
ADR-0002 is untouched. `PALETTE_LEN` rises from 16 to 64, one uniform of 1 KiB,
and a registration past it is `ModError::PaletteFull`, which is the 56 in
decision 4. A Material with no `appearance` row gets the missing-texture magenta
on purpose.

### 7. Running a Mod: a tick in, a batch out

After load a Mod has one entry point:

```wit
record tick-context {
    tick: u64,
    dt-s: f32,
    edits: list<edit-event>,        // player Edits since the last call
    last: batch-report,             // what became of the previous batch
}
record edit-event { at: vec3, before: voxel, after: voxel }
record batch-report { applied: u32, rejected: u32 }
tick: func(ctx: tick-context) -> list<command>;
```

One call per Server tick at most, and **never a second while one is running**.
The events are the Player's Edits only: a Mod's own Commands are not fed back to
it, which would let a Mod loop on itself and costs the contract nothing a Mod
cannot track itself. Each event is a whole Edit, so a batch of N Edits is one
crossing and a list, not N callbacks.

There is no read of the World. A Mod that reacts to a placed Voxel gets it in
`edits`; one that wants to look at a region has nothing to ask. That is a limit
on purpose for this step and the first thing the next one will want (Known loose
ends).

### 8. Commands, and the queue into the tick

```
variant command { fill(fill), place(list<placement>) }
```

Both address Voxels in **metres**. The host turns a point into a Voxel with
`WorldSettings::voxel_at`, which floors, so a Voxel belongs to a region if its
lower corner is in `[min, max)`: the convention worldgen already samples by, and
the same set at any Voxel size. A region is therefore a physical thing, and the
number of Voxels it holds depends on `voxel_size`.

Validation is on the host, per batch, before anything is queued, and rejects the
batch (counted in the next `batch-report`, logged once with the Mod's name) and
not the Voxel:

- finite coordinates; `min <= max` per axis; a region no larger than
  `MAX_FILL_VOXELS` (32 768, a 32³ block) **in Voxels at this World's size**.
  This is a cost cap, not a size, and the one place a Voxel count appears; at
  0.25 m a fill covers an eighth of the span it does at 1 m and the Mod learns
  that from `world-info`;
- at most `MAX_COMMANDS` (1 024) commands and `MAX_VOXELS_PER_CALL` (65 536)
  Voxels in total per call;
- `material` known to the table, `shape` known, `orientation < 24`, no air
  Place: the same checks `edit::resolve` makes for a player's Place, shared and
  not copied.

A Voxel whose Chunk is not held is skipped when it is applied (the Server holds
no Chunk for a Mod), counted in `rejected`. The shaped-cap check is
`set_voxel`'s own.

**The queue.** `PendingCommands` is `pub(in crate::server)`, like
`PendingEdits`, and only `server::mods` can push to it, so the Client cannot
(ADR-0002) and a Mod cannot skip validation. A drain system runs in
`FixedPreUpdate` straight after `apply_pending`, takes up to
`MOD_VOXELS_PER_TICK` Voxels, round-robin across Mods in load order so one
cannot starve another, and writes every Voxel through **`edit::set_voxel`**. A
fill keeps a cursor and is expanded lazily, so a 32 768 Voxel fill is a few
thousand queue bytes and not a vector of Voxels. A Mod whose backlog passes
`MAX_BACKLOG_VOXELS` (262 144) has its `tick` skipped until it drains:
backpressure and not loss, and a Mod that never drains is visible in the log.

Why `set_voxel` and not a bulk write: it is the one place the shaped cap, the
incremental relight, the stale mesh marks and the level-1 cover invalidation are
all done, and a second writer is the mistake ADR-0002 exists to prevent. The
cost accepted is the budget: `MOD_VOXELS_PER_TICK` starts at 256, so a full 32³
fill takes 128 ticks, two seconds at 64 Hz. Ticket 07 measures what a Voxel
through `set_voxel` costs under a relight and sets the number from that; a bulk
path that relights per Chunk is the named follow-up, and it has to arrive
through the same function's invariants and not around them.

### 9. Where Mod code runs: a Job, driven asynchronously

Mod code runs **off the tick**, as a Job on the `AsyncComputeTaskPool`
(ADR-0007). A Mod's `Store` and instance are the Job's owned state: moved into a
task when a call starts, moved back out with the result, and landed on the next
tick that finds the task finished. The events go in by value (a cloned
`Vec<EditEvent>`, small) and the batch comes out by value. No `&mut` into World
state is on a worker, and nothing a Mod calls can reach the `ChunkStore`.
`tesserae-mods` has no scheduler: it exposes `async fn` entry points over a
`ModInstance`, and `tesserae` spawns them, which is the same split ADR-0007 made
for `tesserae-core`.

The calls are **`call_async`**, not synchronous ones, and that is a sandbox
decision before it is a performance one. A synchronously driven WASI `poll` or
sleep blocks the worker thread inside the host, where neither fuel (no guest
instructions run) nor an epoch (the guest is not running) can interrupt it; an
async one is an await point, and dropping the future cancels the call. So the
deadline in decision 10 is a `select` between the call and a timer, and a Mod
that sleeps for an hour is a Mod that missed its deadline.

**This is the one claim in the spec that is argued and not yet shown**, and
ticket 04's first test is the proof: a guest that calls `std::thread::sleep` for
an hour must be cancelled at the deadline with the pool's other workers
unaffected. If it cannot be, the fallback is a dedicated OS thread per Mod with
epoch interruption and `wasi:io/poll` and `wasi:clocks` linked as traps, and
decision 9 is rewritten before anything depends on it.

One call in flight per Mod, and one pool: Mod Jobs take workers from streaming
(the same three), so `fuel_async_yield_interval` makes a long call give a worker
back every `YIELD_FUEL` instructions instead of holding it for the call.

### 10. The sandbox

An Engine is built once with, and each setting traces to the sources above:

- `wasm_component_model(true)`; no component-model async features (marked very
  incomplete), `wasm_threads(false)`, `wasm_memory64(false)`, and
  `wasm_relaxed_simd(false)` so a Mod's float results do not depend on the host
  CPU.
- `consume_fuel(true)`. Each call is given `CALL_FUEL` and yields every
  `YIELD_FUEL`. Fuel and not epochs, because it is exact: the same Mod in the
  same state makes the same decisions on a slow machine and a fast one, which
  makes a Mod testable. The overhead is the price and ticket 05's bench puts a
  number on it.
- A `StoreLimits` per Mod: `memory_size` 64 MiB, `table_elements` 10 000,
  `instances` 4, `memories` and `tables` 4, and `trap_on_grow_failure(true)`, so
  a Mod that runs out of memory is a trap now and not an abort later. Stack
  stays at wasmtime's 512 KiB.
- **A wall-clock deadline of `MAX_CALL_WALL`** per call (decision 9). Over it
  the future is dropped and the Mod is disabled.
- **WASI is linked and empty.** A Rust `std` guest imports WASI, so the Linker
  gets `p2::add_to_linker_async` over a `WasiCtx` with no preopened directory,
  no environment, no arguments, no network, stdin closed, stdout and stderr
  captured and forwarded to the log by line with a rate limit, and both random
  sources seeded from the World's seed and the Mod's id, so a Mod run twice in
  the same World does the same. A Mod can read the clock; it cannot learn
  anything from it that the Server does not already allow.
- **A fault disables the Mod**: a trap, out of fuel, a missed deadline, a batch
  that fails validation more than `MAX_BAD_BATCHES` times in a row. Its Store is
  dropped (a trapped Store cannot be entered again), a `ModDisabled` message
  carries the name and reason, and Commands already queued still apply. A
  disabled Mod stays off until the World is reopened; a restart is not in this
  step.
- **The host trusts nothing it is handed.** Every list length is checked as it
  is lowered, every Material, Shape and Orientation re-checked at apply, every
  float tested for finiteness. The numbers (`CALL_FUEL`, `YIELD_FUEL`,
  `MAX_CALL_WALL`) are placeholders that ticket 04 sets by measurement: fuel per
  millisecond on the M5, so that a full `CALL_FUEL` is about 5 ms and
  `MAX_CALL_WALL` is a generous multiple. They live in one `Copy` struct,
  `ModLimits`, which a Manifest may lower and never raise.

Ticket 04's fixtures, each a small Rust guest that must be contained: an
infinite loop, an hour's sleep, a 1 GiB allocation, unbounded recursion, a panic
(abort), opening a file, reading an environment variable, a TCP connect, a
declaration of ten thousand Materials, and a batch of ten million Commands.

### 11. Trust

A Mod is native-speed code from the internet. The sandbox decides what it can
do; Trust decides whether it runs at all.

- A trust record is `(id, SHA-256 of the component bytes and the Manifest
  bytes)`, in a `trust.json` beside the mods directory. A Mod whose id or hash
  is not in it is **not compiled**. Any change to the file or the Manifest asks
  again, so an updated Mod is a new decision.
- The prompt is a **terminal prompt** in `main`, before the `App` exists,
  because the Server needs the answer to open the World and there is no UI state
  machine to put a window dialog in. It prints the id, version, hash, whether it
  is a Game, and in plain words what a Mod can do (add Materials and Shapes;
  from a Game, change World generation and the view distance; place and remove
  Voxels in loaded Chunks in batches; nothing else: no files, network or
  environment) and the limits it runs under, and reads `y/N`. The window prompt
  is a loose end.
- Unattended runs take `--trust-all` or `TESSERAE_TRUST_MODS=1` and print that
  they did. CI uses the latter; a shipped build has no default of yes.
- DESIGN.md's Look pack caveat (WGSL can hang the GPU) calls for the same
  prompt. The record has a `kind` field, `"mod"` now, so a Look pack is a second
  kind of entry and not a second mechanism.

### 12. Crates and boundaries

`crates/tesserae-mods` is added to the workspace.

- **Depends on** `tesserae-core` (for `ShapeDef`, `MaterialDef`,
  `WorldgenParams`, the ids) and on `wasmtime`, `wasmtime-wasi`, `toml`,
  `serde`, `sha2`, `dirs`. **No Bevy, no wgpu**, enforced by the Boundaries step
  of `./scripts/cargo-ci` and CI, which already loops over `tesserae-mods` and
  skipped it as absent: this step makes the check real. `deny.toml` gains a
  `[[licenses.exceptions]]` entry for `tesserae-mods` as for the other two.
- **Owns** the WIT, the bindgen output, the Engine and its limits, Manifest and
  Trust, the Registry, the conversion from a Declaration to core types, and the
  Command types with their validation. It exposes no `wasmtime` type: the
  boundary the rest of the engine sees is `ModHost`, `ModInstance`,
  `Declaration`, `CommandBatch` and plain errors, so `tesserae` can be built
  without learning wasmtime's API, and the wasmtime version is one `Cargo.toml`
  line.
- **`tesserae`** gains `server/mods.rs` (the plugin: load result into resources,
  the Job dispatch and landing, `PendingCommands`, the drain) and a small
  `client/palette.rs`. The Client imports nothing from `tesserae-mods` but the
  palette's plain data.
- **`tesserae-core`** gains `WorldgenParams::validate` and nothing else, no
  dependency, no WASM.
- `wasmtime` features: `runtime`, `cranelift`, `component-model`, `async`,
  `parallel-compilation`, `std`; default features off, so no `wat`, `cache`,
  `profiling`, `coredump`, `gc` or `threads` that a Mod host does not use. The
  pooling allocator is left off: it is for many short instances, and a World
  loads a handful of long-lived ones.

### 13. `games/demo`

Excluded from the workspace already (root `Cargo.toml`), so a root build does
not compile it for the host. It is a `cdylib` for `wasm32-wasip2` with its own
`Cargo.lock`, `wit-bindgen` for the guest bindings pointed at
`../../crates/tesserae-mods/wit`, a Manifest with `root = true`, and **MIT**
licensed (USE-GRANT.md already says `games/` is not BSL). The demo may use only
the contract; ticket 09 greps its manifest and source for any path into
`crates/` other than the `wit` directory.

It does something visible and small: declares a Material (`demo:lamp`, an
emitter) and the dev fence's Shape as data, sets a World with a lower relief and
no water in `world`, a `view` of 160 m and 3 000 m, and on its first `tick`
returns a `fill` and a `place` that build a short wall and a lantern post beside
the spawn point. That exercises every part of the contract once and nothing
more. `cargo run` finds it if the user's mods directory or `--mods` points at
`games/demo/dist`, which a `scripts/build-guests` step produces.

CI builds it (`rustup target add wasm32-wasip2`, `cargo build --release --target
wasm32-wasip2 --manifest-path games/demo/Cargo.toml`) and a test loads the built
component, runs `declare`, one `tick`, applies the Commands to a headless
`ChunkStore` and asserts the Voxels. "Engine changes that break content fail CI"
is ADR-0004's promise and this is how it is kept.

The dev build registers a fence of its own at `FENCE_ID` for the gallery and
hotbar. With a Game loaded that would register a second fence at a different id,
so under `dev` it is registered only when no Game is loaded.

### 14. Out of scope

- **Look pack WGSL**, material extensions and post-process settings (step 9).
  `appearance` is the stopgap; the trust record's `kind` is the only provision.
- **Entities**, AI, recipes, inventory: nothing in the contract names them.
- **Network**: a Mod runs on the Server, and in step 10 the Server is the thing
  that is remote. Commands are already a message, which is the only provision.
- **A save file**, so the Registry is not written, and a missing Mod is an
  error.
- **Reading the World from a Mod**, **callbacks per Voxel** and **reloading a
  Mod at run time**. The tables are immutable once the World is open (ADR-0007),
  so a reload is a new World.
- **Publishing the WIT** to a registry.

## Invariants, and where each is enforced

1. **No per-Voxel host call** (ADR-0001). The import surface is two functions
   and WASI; a test lists a built component's imports and fails on any other.
2. **No Mod reaches `tesserae-core`** (ADR-0001). A Mod's data is converted in
   `tesserae-mods` and handed to `register`; there is no path from a guest to a
   table or a Chunk. Commands reach Voxels only through `set_voxel`.
3. **No Client system mutates World state** (ADR-0002). `PendingCommands` and
   the drain are `pub(in crate::server)`; no `client/` file imports
   `server::mods`; the only new Client input is the palette's plain data.
4. **`tesserae-core` and `tesserae-mods` depend on neither Bevy nor wgpu.** The
   Boundaries step, now covering both crates.
5. **Everything physical is in metres** (ADR-0003). Positions, regions, the
   reach and every World parameter in the contract are metres; the only Voxel
   counts are cost caps, and a test runs the same Command at 1 m and 0.25 m and
   asserts the same physical region.
6. **A Job holds owned state and lands on the tick** (ADR-0007). `ModInstance`
   moves into the task and back; no World resource is in a task's signature.
7. **The tables are immutable after load.** `ContentTables` is built before the
   `App` and never written again.
8. **A Mod cannot outlive its limits.** Decision 10's fixtures.

## Performance

`crates/tesserae-mods/benches/host_calls.rs`, criterion, `harness = false`,
built by CI's bench step without measuring. Load average is checked with
`uptime` before any run and recorded; a run above a load the machine can absorb
is not reported, and a comparison is interleaved CPU time and not wall clock.

| Bench                    | What it isolates                                |
| ------------------------ | ----------------------------------------------- |
| `host/tick_empty`        | The floor: one `tick` returning nothing.        |
| `host/tick_commands/N`   | A guest returning N `place`s (16, 256, 4096).   |
| `host/validate/N`        | The same decode and checks natively, no WASM.   |
| `host/declare`           | Load: 8 Materials and 8 Shapes.                 |
| `host/instantiate`       | `instantiate_pre` plus instantiate, per Mod.    |
| `host/naive_per_voxel/N` | A bench-only guest calling `set-voxel` N times. |

The last row is ADR-0001 as a number. It uses a second, bench-only WIT world
that the shipped Linker never defines, and exists to say how many times worse
the design this contract refuses would be; a test pins that the shipped world
has no such import. The expectation to check is that `tick_commands` is flat per
command in N and that `tick_empty` is small against the 15.6 ms tick, not a
particular figure.

The tick-side cost is measured in ticket 07 with the `dev` budget sweep that
earlier tickets used: the worst main-thread tick with a Mod applying fills
against without one, and `MOD_VOXELS_PER_TICK` set from it.

## Known loose ends, declared rather than discovered

- **A Mod cannot read the World.** The next thing a Game wants. It needs a
  Snapshot handed to the call, which ADR-0007 already has the shape of, and a
  decision on a region read as one batched crossing and not a query per Voxel.
- **No bulk write.** Commands go through `set_voxel`, so a large fill takes many
  ticks. A per-Chunk write that relights once is the follow-up.
- **No save, so no tombstones.** A missing Mod is `RegistryMismatch`. A save
  step decides how a hole in a dense id space is held (decision 4).
- **No compile cache.** Every launch compiles every Mod.
- **The trust prompt is on the terminal.** A window prompt needs an app state
  before the World opens.
- **Appearance is a 64-entry flat palette** until step 9.
- **A disabled Mod stays disabled** until the World is reopened.
- **Mod Materials at distance** are what phase 6 left them: a dominant Material
  by volume, no emission.
- **The contract's `command` and `occlusion` are closed.** Growing either is
  `0.2.0`.
- **The export-version question in decision 2** if the test shows it needs a
  fallback.

## Done when

`cargo run` with `--mods games/demo/dist --trust-all` opens a World with the
demo's terrain, its lamp and fence Shapes in the hotbar-visible tables, its
reach, and a wall and a lantern post built at the spawn by a Mod, shown in a
screenshot taken with `TESSERAE_SCREENSHOT_OFFSCREEN=1` and shown to the user; a
Mod that loops, sleeps, allocates, recurses, panics or asks for the file system,
network or environment is contained and named in the log; `./scripts/cargo-ci`
passes in full including the Boundaries step for both crates and cargo-deny; the
host-call benches are reported with the load average beside each figure; and
DESIGN.md (step 7, Open items), `docs/architecture.md`, `CONTEXT.md`, ADR-0012
and the assets note in ADR-0009 are updated. If part of the scope is blocked,
the rest is finished and the PR says exactly what was left out and why.

## Outcome

Written as the tickets finish; what changed from the text above, and why.

### Ticket 01: the contract

- The package is `tesserae:mods`, not `tesserae:mod`: wit-bindgen 0.57 and
  0.62 do not escape a package name that is a Rust keyword, so a guest in
  `tesserae:mod` does not compile. The world is still `mod`. Decisions 1 and 2
  above are corrected to match.
- Export lookup is semver-compatible like import lookup: a guest built against
  0.1.0 instantiates on a host defining 0.1.1 and its exports are found under
  both names. The open question of decision 2 is closed and no fallback was
  written. A host built against a newer patch must look up `@since` functions
  optionally, through the export index, not through the generated
  `instantiate`.

## Tickets

| #   | Title                                              | Blocked by |
| --- | -------------------------------------------------- | ---------- |
| 01  | The crate, the WIT package and the guest toolchain | -          |
| 02  | Engine, Manifest and Trust, headless               | 01         |
| 03  | Declarations into tables, and the Registry         | 01         |
| 04  | The sandbox, and the fixtures that test it         | 02         |
| 05  | The tick, Command batches and the host-call bench  | 03, 04     |
| 06  | The Server loads Mods before the World opens       | 02, 03     |
| 07  | Mod Jobs, the Command queue and its drain          | 05, 06     |
| 08  | The trust prompt and the mods directory            | 06         |
| 09  | games/demo, built in CI                            | 07, 08     |
| 10  | Docs, ADR-0012, measurements and the PR            | 09         |
