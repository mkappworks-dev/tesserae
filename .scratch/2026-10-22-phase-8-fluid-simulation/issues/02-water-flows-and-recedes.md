# 02: Make placed water flow and recede

Status: done
Blocked by: 01

Spec: [decisions 2, 4, 5, 6 and 11](../spec.md).

## What to build

Deliver a full-detail waterfall and shallow stream from an explicit water
source, plus source controls that make the flow reversible. Run the native
kernel as bounded worker work and land only on the Server.

## Acceptance criteria

- [x] Generated and explicitly placed water is source water; simulated flow
  never becomes a source or fills a Cave upward through pressure.
- [x] Flow prefers downward travel and spreads sideways over metre-valued
  reach with diminishing surface height. Successive physical drops can renew
  reach; a horizontal surface slope cannot renew it.
- [x] Reset-drop distance and propagation/recession rates use metres and
  simulation seconds. Equal physical fixtures at 1.0, 0.5, and 0.25 m Voxel
  sizes respect the documented quantization tolerance.
- [x] Cutting supply and removing sources removes unsupported flow and cycles.
  Blocking a stream does not delete its upstream source.
- [x] Explicit fluid-tool intents place/remove sources using Server-validated
  rays, origin/reach checks, and the existing queued-intent lifecycle.
  Removal preserves solid state; refusal changes neither layer.
- [x] The ordinary solid Edit ray retains its behavior. Placeholder controls
  and feedback make source placement/removal exercisable without inventory.
- [x] Full-detail geometry and lighting follow changed water state. Addition
  can darken and removal can brighten; an older Job cannot resurrect a source.
- [x] The native kernel's quota/continuation and controlled Server tests
  demonstrate a complete single-region flow/recession cycle.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Draft initial tuning and the physical reset-drop rule are proposed in the
  spec; they are not fixed counts of Voxel steps.
- 2026-10-22: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- `tesserae-core/src/flow.rs` is the kernel. `advance(view, tables, scale,
  continuation, quota) -> Advance` reads stored cells through `CellView`
  (`Region` of shared `Arc<Chunk>`/`Arc<ChunkOverlay>` handles, or a test
  grid), never writes, and returns `Partial(Continuation)` when the quota
  runs out or `Complete(Finished { proposals, wakeups, timers, work })`.
  `Proposal { at, read, next }` carries the `Occupants` it was computed from.
  `Wakeups` is the coalesced (earliest step wins) set of Voxels to evaluate,
  `take_due(step)` hands them out in coordinate order, `wake_around` wakes the
  ten Voxels whose state reads a changed one (`READS` reversed). `Timers`
  travel with the continuation. `FlowScale::new(fluids, voxel_size_m)` turns
  each `FluidDef` into whole Voxel steps of reach, the reset drop in
  sixteenths of a Voxel and per-Voxel delays in steps of `STEP_S` (1/64 s,
  the Server tick).
- Rules: a source is fixed; flowing fluid is the best offer of its
  neighbours. The fluid above falls in (adding a Voxel of descent; descent of
  the reset drop renews reach, keeping the remainder modulo the reset); a
  side that is a source, or flowing and unable to fall, spreads in with one
  step less reach and its descent unchanged. Nothing offers upwards. An
  existing fluid admits offers of its own Material only. Ties: reach, then
  descent, then falling, then the lower Material id. Surface is full when fed
  from above, else `1 + 15 * reach / max_reach` sixteenths. Every offer goes
  down a level or loses reach, so unsupported flow, rings included, always has
  a highest member nothing supports, and recession needs no global pass.
- Time: a change wakes its readers for the next step; a Voxel that wants a
  different state starts a timer of the edge's delay (downward, sideways, or
  recession for a worse state) and changes when it expires; a different
  wanted state restarts it. All evaluations of a step read the state at its
  start and proposals are held until the step is complete, so quota division
  cannot change results. Unknown terrain (`None`) pauses the Voxel.
- `Fluid` gained `reach` (Voxel steps left) and `descent` (sixteenths since
  renewal), zero for a source; `Overlay` is 12 bytes, so one wet Voxel's
  overlay is 4096 + 2 x 12 bytes. `vet_occupants` now also refuses a source
  that is not full or carries flow state (`BadSurface`), keeping the packed
  encoding canonical.
- Light: `optical(voxel, overlay)` / `optical_chunk` read fluid in empty space
  as a cube of its Material (shared per-Voxel light). `LightLattice::voxel`
  and the bulk solve in `jobs::solve` read it, and `set_occupants` relights
  whenever the optical reading changes, so water flowing into air darkens and
  receding brightens though the packed Voxel stays air. This replaces ticket
  01's "an overlay-only change does not relight".
- Meshing: `LitChunk::with_overlay`; `Neighborhood` puts full-surface flow in
  as a cube of its Material and keeps partial surfaces in a side array
  allocated only when one of the seven overlays holds fluid; a third pass
  draws each partial cell as a box to its surface (sides hidden against a
  covering neighbour or same fluid at least as high, only the step shown
  against lower fluid), merged with pass 2's machinery. `mesh_chunk` no
  longer declines a uniform-air Chunk whose overlay holds fluid. Mesh jobs
  pass overlays.
- Server: `server/fluids.rs` runs one logical step at a time on the
  `Stage` worker machinery: due Voxels and Snapshots of the Chunks they read
  go out with the continuation (1024 evaluations per call), a `Partial` is
  re-dispatched next tick, a `Complete` step's proposals land through
  `set_occupants`, at most 64 per tick, and the next step starts only once
  all have. At most one step completes per tick. `ChunkStore` journals every
  position `set_occupants` changes; the system drains it and wakes the
  dependents for `now + 1`, so player Edits, the Mod drain, dev builders and
  landed flow all wake the simulation through one path. A proposal is
  dropped and its Voxel re-woken if an Edit since the step's first Snapshot
  touched anything it read, if its Chunk's lifetime changed, or if the Voxel
  no longer reads `read`; landing into a Chunk whose light is being solved is
  deferred a step. The system runs in `FixedPreUpdate` after
  `edit::apply_pending` and the Mod drain (new `mods::Drain` set).
- Fluid tool: `EditAction::PlaceSource(MaterialId)` and `RemoveSource` go
  through the existing `receive`/`apply_pending` lifecycle (origin tolerance,
  reach from the Eye, unlit deferral). Their ray (`raycast_until`, of which
  `raycast` is the solid case) stops at solids and packed sources and passes
  flowing water. Placing puts a source in front of the struck face if that
  Voxel has no solid and no source or other fluid (flow of the same fluid is
  replaced); removing needs a source at the struck Voxel and clears only its
  fluid, so a wet stair keeps its stair. New `RejectReason::NoSource`. Player
  history records the 0.1 Voxel projection, so a fluid intent that leaves the
  packed Voxel alone is no event.
- Client: Alt+RMB places a water source, Alt+LMB removes one; the HUD hint
  says so and rejections use the existing notice.

### Verification

- Core: 237 lib tests (221 before), 60 meshing integration tests (57).
  Server: 294 in the `tesserae` binary, one ignored as before (282). Mods:
  74, unchanged. Workspace 665 (634).
- Kernel tests, each at 1.0, 0.5 and 0.25 m where a size matters: the scale
  (7 m is 7/14/28 steps; reset 8/16/32 sixteenths; 16/8/4 and 8/4/2 steps);
  spread is exactly 7 m with non-increasing surfaces and one source; the
  front is 4t m out at t = 0.25..1.5 s within one Voxel; recession of a
  stream completes in 7 m / 4 m/s within one recession delay; a 4 m
  waterfall falls full and spreads 7 m again from its foot; 1 m terraces 4 m
  long carry a stream 19 m and more; a 0.25 m step at 0.25 m keeps the
  remaining reach while 0.5 m and 1 m drops renew it; a U and the far arm
  stay dry above the bottom; a ring around a pillar clears when its source is
  removed; blocking keeps the source and dries downstream; quotas 1, 3 and
  17 give the same state at every one of 400 steps (with a block and a
  removal mid-run) and the same work counts; a call never exceeds its quota;
  unknown terrain pauses; settled flow does no work over 500 steps; solids
  block flow; a shaft dug under a generated lake (default `Worldgen`, seed 7)
  fills from it, no flow becomes a source and nothing rises above the
  waterline.
- Meshing tests: flowing water in the overlay meshes a six-face box to its
  surface, full flow as a cube; between two flows only the step shows and a
  floor hides the bottom; a dry overlay meshes identically to none.
- Server tests, controlled ticks over the real `apply_pending`, `receive` and
  `simulate` systems and the async pool: the full place/flow/remove/recede
  cycle at three sizes (7 m, one source, 1.75 s plus at most four steps,
  floor kept, nothing stale); quota 2 and one landing per tick against the
  defaults give identical state at every logical step and identical work
  through a flow and a recession, with more worker calls; a held worker that
  read the source while the player removes it never lands a Voxel of water
  beside it, and the stream recedes; blocking with the ordinary Edit keeps
  the source; refusals (no source, not a fluid, removal on flow) change
  nothing; a lantern's light through the corridor dims when water flows in
  and returns when it recedes, equal to a fresh `light_chunk` of the optical
  cells both ways; in the streaming harness a corridor cut in deep rock is
  remeshed and redelivered as water flows and recedes, each time equal to a
  fresh mesh of the store with overlays and with no water quads at the end.
  Edit tests: the fluid tool places in front and removes keeping the bed and
  a stair; a source replaces flow and the tool aims through flow; refused
  fluid intents (no source, not a fluid, sky, too far, flow, origin too far)
  change no layer; the solid ray goes through flowing water. Client test: Alt
  maps the buttons to the fluid tool, without it they keep Break and Place.
- Mutation checks, each failing exactly the test named: dropping the
  "an Edit touched what it read" landing check (older job test); relighting
  only on a packed-Voxel change (light test); renewing reach on every drop
  (reset-drop test); mesh jobs without overlays (streaming mesh test);
  timers that do not wait (front-rate test).
- Every pre-existing test passes unchanged except where a `Fluid` literal
  gained the two fields and the `Overlay` size assertion (8 to 12 bytes). Full
  `scripts/cargo-ci` passed on the tree this commit records.
- No Client mutator: `FluidSim` and every new store method that writes are
  `pub(in crate::server)`; the Client only sends `EditIntent`s. Core gained no
  dependency; `Cargo.lock` is unchanged.

### Deviations, limits and what later tickets inherit

- Time quantization: each Voxel's delay is rounded to whole steps, at least
  one. Water's tuning at 1.0, 0.5 and 0.25 m divides exactly, so the fixtures
  agree to the step; a rate that does not (lava's 0.75 m/s is 85.3 steps per
  metre) accumulates up to half a step per Voxel travelled, more than the
  spec's one quantum over a long path. Ticket 05 or 09 should carry a
  sub-step remainder if that matters for declared fluids.
- Recession in a stream counts down: each Voxel takes the next best offer,
  so surfaces fall progressively; a stream clears in about reach over the
  recession rate. A timer restarts when the wanted state changes, so a Voxel
  whose inputs change faster than its delay forever would never change; not
  seen in any fixture, worth a look under ticket 03's saturation tests.
- For ticket 03: one fluid job at a time; paused evaluations (unknown
  terrain) are counted and dropped, not retained; Chunk admission and
  eviction wake nothing; `Wakeups`, `Timers` and the store's change journal
  are coalesced per Voxel but not count-bounded; the light-unlit deferral
  and stale drops re-wake for the next step. Flow crosses held Chunk
  boundaries already (the Server corridor spans Chunks -1 and 0) but no
  six-face or admission-order test exists yet.
- For ticket 04: a Voxel with a solid is fixed to the kernel (a wet stair
  neither fills nor recedes), and the fluid tool refuses to place into one.
- For ticket 05: player history for fluid intents is the 0.1 Voxel
  projection only.
- For ticket 08: level 1 still derives from packed Voxels, so flowing water
  is absent from it.
- For ticket 09: `EVALUATIONS_PER_JOB` (1024) and `LANDED_PER_TICK` (64) are
  unmeasured. No hot-path bench was rerun; the dry paths changed by a palette
  scan of the seven overlays per mesh and an overlay lookup for each air
  Voxel the incremental relight reads.
- No live capture was taken; controls are covered by the input test and
  state by the tests above.
