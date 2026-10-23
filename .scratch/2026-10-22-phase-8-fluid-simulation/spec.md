# Step 8: fluids, waterlogging, reactions, heat and fire

Reviewed 2026-10-22; the tickets under `issues/` are triaged ready-for-agent
and implementation proceeds through them in order. A push or PR remains
separate authorization.

## Problem Statement

Water is currently static terrain. Opening a lake into a Cave changes the
terrain but produces no waterfall. Placed water does not spread, and a solid
Shape and water cannot occupy the same Voxel. Games cannot declare native
fluids, contact reactions, or burning behavior.

Step 8 makes these interactions work on the Server, predictably and within
bounded work. A player should be able to divert a stream, waterlog a stair,
observe lava lighting a Cave, cool lava through a declared reaction, and put
out burning Material with water.

## Solution

Use permanent sources and dependent flowing fluid. Sources feed downward
flow and bounded sideways spread; removing supply makes dependent flow
recede. Water does not create new sources or fill connected Caves upward
through pressure. Loaded terrain alone participates in simulation.

Include waterlogging, lava, Mod-defined fluids, declarative contact reactions,
local heat exposure, fire spread, fuel consumption, residue, and extinguishing.
Mods declare the rules as validated data. Expensive evaluation remains native
in tesserae-core, and only the Server applies results to World state.

Water shading and Look packs remain step 9. Step 8 includes the geometry,
placeholder appearance, emission, and mesh updates needed to see its behavior.

## User Stories

1. As a player, I want a breached lake to produce a waterfall and a shallow
   stream, so that terrain Edits affect water.
2. As a player, I want sideways spread to stop at a predictable distance, so
   that opening a lake does not fill an entire Cave to the lake's waterline.
3. As a player, I want a stream to continue down successive drops, so that
   flowing water follows descending terrain.
4. As a player, I want dependent flow to recede when supply is blocked or
   removed, so that diverting water changes the downstream landscape.
5. As a player, I want placing water to create an explicit source, so that
   flowing water never silently multiplies into permanent sources.
6. As a player, I want to remove a source without removing terrain behind
   it, so that placing water is reversible.
7. As a player, I want water and a partial Shape to coexist, so that stairs,
   slabs, posts, and fences can be waterlogged.
8. As a player, I want solid geometry to block passage through its occupied
   space, so that water does not leak through a wall or a sealed Shape.
9. As a player, I want waterlogging to preserve collision and selection, so
   that adding water does not change the underlying solid Shape.
10. As a player, I want lava to flow more slowly than water and emit light,
    so that its behavior and state are apparent.
11. As a Game author, I want to declare fluid speed, reach, heat, and optical
    properties, so that my Game can supply fluids beyond water and lava.
12. As a Game author, I want to declare fluid contact reactions, so that
    water/lava interaction does not hardcode one Game's crafting rules.
13. As a Game author, I want flammability, ignition delay, burn duration,
    residue, and heat output to be data, so that combustion is moddable.
14. As a player, I want exposed flammable Material to ignite near a hot source,
    so that lava and fire interact with their surroundings.
15. As a player, I want solid barriers to block heat exposure, so that wood
    behind a stone wall does not ignite by heat conducting through stone.
16. As a player, I want burning to consume fuel and stop when fuel runs out,
    so that temporary fire cannot sustain itself without a declared source.
17. As a player, I want water to extinguish burning, so that I can control fire.
18. As a player, I want Chunk loading to wake blocked flow and heat work, so
    that an unloaded neighbor is a pause rather than a permanent simulation
    error.
19. As a player, I want Edits and streaming to keep progressing during a large
    simulation, so that fluid and fire work cannot monopolize the tick.
20. As an engine developer, I want repeatable native simulation tests, so that
    worker scheduling does not determine simulation results.
21. As an engine developer, I want uniform and dry Chunks to retain cheap
    storage and idle behavior, so that adding simulation does not make all
    terrain expensive.
22. As a Mod author, I want existing 0.1 components to keep loading, so that
    fluid additions do not silently invalidate the completed Mod host.

## Implementation Decisions

### 1. Approved behavior and proposed engineering choices

The user approved source-based flow, no automatic source multiplication, no
pressure filling, pause at unloaded boundaries, discard on eviction, and
deterministic bounded work that slows under load. The user explicitly included
waterlogging, lava, Mod-defined fluids, reactions, heat, fire, and burning.
The user approved local heat exposure and solid barriers blocking it.

The following representation, compatibility, ordering, and initial tuning
choices are the planner's recommendations. They are presented for approval
with this document, not attributed to prior user agreement. Internal type
layout and algorithm choice remain with the implementer where this document
specifies an observable contract instead.

### 2. Authority and native simulation interface

The Server owns authoritative solid, fluid, heat-exposure, and burning state.
No Client mutator or per-Voxel Mod callback is introduced. Core retains no
Bevy, wgpu, executor, or World scheduler dependency.

A native simulation module accepts immutable cell Snapshots, immutable content
tables, explicit logical time, continuation state, and an operation quota. It
returns owned change proposals, wakeups, continuation state, and work counts.
The same interface serves tests, benchmarks, and worker Jobs. Shape geometry,
fluid state, burning, and light interpretation must not be independently
reimplemented by each caller.

The Server validates and lands proposals through a shared World-change path.
That path preserves the existing placement validation, normalization, shaped
cap, light correctness, mesh invalidation, and level-1 cover invalidation.
Fluid-only and burning-only changes are authoritative changes even when the
packed solid Voxel is unchanged.

### 3. Representation and waterlogging

Preserve the packed Material/Shape/Orientation Voxel and permanent ids. Do not
encode flow levels in Orientation, reserve Shape ids for flow levels, widen
the Voxel, or renumber existing Materials to make room for lava.

Represent fluid and combustion state as additional native Chunk state, with
shared immutable Snapshot ownership alongside solid state. Each occupied
fluid region has its Material, explicit source/flowing status, surface state,
and propagation state. Burning records whole-Voxel fuel progress separately
from the solid Shape. Heat exposure is local transient ignition state, not a
World-wide temperature field. This phase does not subdivide one solid Shape
into independently consumable fuel regions.

There must be one canonical cell interpretation, including for generation,
legacy Commands, picking, light, meshing, and LOD. Adapters may project that
state into old interfaces; they must not maintain two independently mutable
copies of the same water.

Uniform lake water must remain compact. A dry, unburning Chunk must not gain
a dense fluid, heat, or fire allocation. Storage containers, compression, and
the exact Snapshot bundle are implementation choices, verified by memory
measurements and invariants.

Waterlogging uses the existing Shape geometry at its sixteenth-Voxel
resolution. Derive reusable empty-space and passage information at content
registration. Apply it to the actual Orientation and connected arms. Fluid
must not pass through a sealed face, a jointly sealed interface, or a solid
partition between disconnected voids. A sealed internal cavity is not filled
merely because another part of its Voxel is wet.

One Voxel may hold at most one fluid Material at a time. Separate accessible
void regions may need separate reachability/surface state for that Material;
the native representation must preserve this distinction when geometry
requires it. Different fluids cannot occupy disjoint regions of one Voxel
in this phase. They neither mix nor react through an intervening solid wall.

Waterlogging clips fluid geometry to accessible empty space. It preserves the
solid's Material, Shape, Orientation, collision, and selection. Full solids
exclude fluid. Connected-arm changes wake adjacent simulation and remeshing.
Simulation does not overwrite arbitrary non-solid Mod Materials: occupancy,
waterlogging eligibility, and explicit reactions decide what is replaceable.

### 4. Sources, flow, and physical units

Generated water and explicitly placed fluid are sources. Flow created by
simulation is never a source. Sources persist until an Edit or declared
reaction removes or replaces them; they do not lose volume while feeding flow.

Downward motion has priority for dependent flow. A supported stream spreads
horizontally with diminishing surface height and finite path reach. A source
may feed both downward and sideways flow. A declared physical gravity drop
renews sideways reach, matching the chosen waterfall/stream behavior. There
is no global source-distance bound across successive drops.

Accumulate actual gravity-directed descent in metres, including descent
inside partial Shapes. Renew reach when that descent reaches the fluid's
declared reset-drop distance, initially 0.5 m for demo water and lava. Crossing
a Voxel boundary by itself does not renew reach. A lower rendered surface
caused by horizontal attenuation is not gravity descent and cannot renew
reach on flat ground. Any distance remainder is retained after renewal.

Flow cannot climb to equalize connected volumes or raise a Cave pool toward
the lake's waterline. Removing supply must remove self-supporting cycles of
flow as well as simple straight streams. Blocking supply leaves the upstream
source intact and lets downstream dependent flow recede.

Lengths and spatial rates are authored in metres and metres per second. Time
is in simulation seconds. Surface fractions and light levels are
dimensionless; they do not introduce hidden physical distances in Voxels.
Quantization is explicit and finite. At 1.0, 0.5, and 0.25 m Voxel sizes, an
equal physical setup has equal authored reach and propagation speed, within
one Voxel of spatial resolution and one logical time quantum.

Proposed initial water tuning is 7 m of sideways reach, 4 m/s of sideways
propagation, and 8 m/s downward propagation. Proposed demo lava tuning is
3 m of reach, 0.75 m/s sideways, and 1.5 m/s downward. These are independent
physical parameters, not fixed counts of Voxel steps. Recession has a declared
rate too, initially equal to forward propagation. Final defaults must be
reported with the measurement evidence; no accepted default may hide a
Voxel-size dependence.

### 5. Edits and source control

Player and Mod solid placement share the existing validation and shaped-cap
rules. Placing a partial Shape into compatible fluid preserves fluid in its
remaining accessible space. Placing a full solid displaces its fluid; breaking
only the solid preserves any remaining fluid. Neither operation creates a
source from flowing water.

Keep the existing ordinary Edit ray's solid-picking behavior. Add an explicit
fluid-tool intent for placing and removing sources. A modifier on the current
place/break controls is sufficient for the placeholder UI; the Client sends a
ray and action, and the Server checks origin, reach, target, and eligibility.
Do not require a bucket inventory or full interaction redesign.

Source removal clears fluid at the targeted cell without deleting its solid.
Any remaining upstream source may refill it as flow. A refused edit changes
neither layer. Replacing a solid or source also cancels obsolete exposure and
fuel state, preventing a later Job from resurrecting it.

Keep player-only Mod Edit history semantics. Simulation, Mod Commands, and
dev-builder changes do not become player Edits. Extend the new contract's
player history to distinguish solid and fluid intents; the old contract keeps
its existing Voxel projection and receives no synthetic simulation events.

### 6. Determinism, scheduling, and stale work

Use logical simulation time and a stable authoritative event order. Events
include player Edits, drained Mod Commands, and Chunk admission/eviction.
Document which logical step observes each event. Determinism means that equal
initial state and equal ordered events at equal logical simulation steps give
equal state, including fluid, fuel, and exposure, regardless of worker
completion order or work-budget subdivision.

This does not promise equal state at the same rendered frame or wall-clock
Server tick. Streaming admission timing and asynchronous Mod-call timing are
inputs to the event trace, not magically deterministic inputs. Replay tests
control those events explicitly.

Do not advance a dependent step from whichever Job finishes first. Use stable
coordinate and face order, deterministic competition between fluid Materials,
and stable reaction priority. Without a reaction, earlier registered fluid
wins an otherwise equal simultaneous claim on empty space. An existing fluid
blocks a different fluid Material, whether source or flowing. There is no
density ordering or displacement of unmatched fluids. An explicit Edit may
replace fluid; loss of supply may clear it and wake competing neighbors.

The implementer may use rounds or a deterministic event queue, but must expose
explicit continuation and satisfy those behavioral tests. A global restart on
every player Edit is unacceptable. Snapshots and results carry all read
dependencies, including fluid, burning, geometry, and Chunk lifetime. Landing
rejects or recomputes affected stale work without discarding unrelated work or
starving Edits. No partial multi-cell reaction may land.

Bound evaluation attempts, new wakeups, retained pending work, in-flight Jobs,
and landed mutations. Rejected/stale/blocked attempts count as work. A budget
counts operations, not elapsed wall time. Reaching a cap preserves continuation
or applies explicit backpressure; it never silently drops required work.

Use fair service across active Chunks and simulation kinds. Overflow may
coalesce wakeups into a bounded per-Chunk rescan flag, serviced with a cursor;
it must not allocate unbounded event lists. Quiescent fluid and cold,
non-burning terrain do no recurring per-Voxel simulation work. Initial
activation scans and neighbor wakeups are also resumable and budgeted.

Player Edits and the existing fair Mod queues keep progressing while fluid
work is saturated. Do not raise the phase 7 Mod drain cap as a side effect.

### 7. Loaded terrain, persistence, and LOD

Only held level-0 Chunks participate. Missing neighbors are unknown, never
Air. Pause passage, contact reactions, and heat exposure at that boundary.
Chunk admission wakes both sides once usable state is available. It must not
turn all generated sources into a permanently active full-World scan.

Eviction drops local fluid, burning, exposure, and pending work with the solid
Chunk. Reopening/regenerating reproduces generated sources, just as it currently
discards solid Edits. Prevent results for an old lifetime from landing into a
new Chunk at the same coordinate. Held neighbors re-evaluate their dependencies
on load and eviction; no fluid or fire may continue to depend on absent state.

Do not implement save/load in this phase. Define the durable-versus-derived
state inventory for the later save step: solid and fluid state, explicit source
identity, ongoing fuel/exposure progress, content identity, and logical timing
needed to resume. Pending workers, meshes, light caches, and coarse LOD are
rebuildable. This is a persistence design obligation, not a claim that water
already survives eviction.

Level 1 reflects held changed level-0 fluid and burning state and receives
proper cover invalidation. Higher levels retain the generator-only policy of
ADR-0011. Coarse levels never run fluid or fire simulation. Document the
resulting distance discontinuity instead of extending simulation to 4 km.

The existing generated-waterline mesh correction must not relocate arbitrary
flowing or waterlogged surfaces. Coarse fluid surfaces remain a volume-based
approximation. Preserve the established lake-water seam behavior where its
premises hold. Dynamic emitter light at distance keeps ADR-0011's existing
coarse-light limitation; do not add a separate coarse light flood.

### 8. Mod declarations and compatibility

Fluids use registered Material identities, with separate native fluid behavior
data. No new permanent built-in Material id is required for lava: the demo
declares it. Shape geometry and fluid appearance remain distinct from Material
behavior. Registration finishes before the World opens.

The new contract declares fluid reach, reset-drop distance, and rates,
waterlogging eligibility,
heat output/reach, extinguishing behavior, flammability, exposure threshold
and ignition delay, burn duration, burn emission, and residue. References use
qualified names resolved after Declarations, allowing dependency-owned content
without hardcoded numeric ids. Unknown references, non-finite values, invalid
ranges, or contradictory definitions fail load before the World opens.

Use a new 0.2 contract line for changed records and Command variants. Keep
hosting 0.1 components through an explicit adapter; do not label closed-record
or variant changes as a compatible 0.1 patch. Existing guest fixtures remain
real compatibility tests. A 0.1 fluid-Material placement maps to a source;
ordinary legacy solid placement follows the layer rules above.

The new batched Commands can place/remove sources and ignite/extinguish an
eligible target without arbitrary direct access to simulation internals.
Validation, bounded fair queues, lazy region iteration, and batch reports
continue to apply. No World read import or per-Voxel simulation import is
introduced. Player tools and both contract adapters reach one shared native
World-change path.

### 9. Contact reactions

Games and Mods declare fluid-pair reactions as data, executed natively on
actual contact through accessible geometry. Resolve names after content
registration. A rule states the participating fluid pair, which side is
consumed, optional solid result, and source/flowing matching where relevant.

Make pair matching orientation-independent unless a rule explicitly names
the receiving side. Canonicalize mirrored duplicates and reject conflicting
definitions at load. For simultaneous competing contacts, process stable
coordinate/face order; once a reaction consumes a participant, later proposals
must validate again. A result never depends on hash iteration.

Solid products require empty solid space and the same validation and caps as
an Edit. A reaction cannot destroy a coexisting stair or fence to make room
for stone. If a whole result cannot apply, neither participant is consumed.
An unchanged blocked contact sleeps until its inputs change.

The demo declares water/lava contact converting contacted lava into existing
stone while preserving water. No obsidian, cobblestone, crafting, or inventory
system is required. A second custom-fluid fixture demonstrates that the native
mechanism is not a water/lava special case.

### 10. Heat, ignition, fire, and burning

Use the approved local-exposure model. Hot fluids and burning Material emit
bounded heat exposure through reachable open space. Spatial reach is in metres;
solid barriers block passage, including Shape partitions and jointly sealed
interfaces. Heat may take an open route around a barrier within its reach.
It does not conduct through solid stone or store temperatures in all solids.

Use bounded, dimensionless heat strength with a declared ignition threshold
and continuous exposure duration in seconds. Distance attenuates exposure
monotonically over its metre-sized reach. The strongest eligible source
determines exposure; overlapping sources do not sum into an unbounded value.
Document the deterministic attenuation and quantization used by the kernel.
Dropping below the ignition threshold resets incomplete ignition progress.

An exposed flammable Material ignites after its declared delay. Burning
consumes a finite duration of whole-Voxel fuel in simulation time, emits
declared heat and block light, and then replaces the solid Material with its
declared residue while retaining Shape/Orientation, or removes the solid if
the residue is Air. The residue does not automatically burn unless its own
declaration allows it.
Replacing or removing fuel ends its fire. A deliberately permanent source
requires explicit declaration rather than an accidental fire cycle.

Extinguishing fluid touching a Voxel's solid fuel protects the whole fuel from
ignition and extinguishes existing burning, resetting ignition progress. It
touches from anywhere in the Voxel's own accessible voids, or from an accessible
void of a face-adjacent neighboring cell that reaches the shared face where the
fuel's solid covers it: water pooled against a cube's side, standing on its top,
or running past it. A neighboring full cell of fluid reaches the whole shared
face. This deliberately follows a whole-Voxel wet-fuel rule: contact on any one
face, or a wet part, protects a dry part of the same fuel, and fuel consumption
never requires partial destruction of its Shape. Fluid must actually reach the
fuel: fluid behind a neighbor's sealed face or partition, or in a void of the
neighbor that does not reach the shared face, does not count, and nothing
suppresses fuel through a solid partition. Flowing and source fluid both count.
A neighbor in terrain that is not loaded never counts as wet, and fuel beside
one decides nothing until it is loaded. Displacing the fluid permits later
reignition after the normal delay. Other fluids' extinguishing behavior is
declarative. Heat and extinguishing obey loaded-boundary and geometry rules.

Proposed demo settings use 2 m heat reach, a 2 s ignition delay, an 8 s burn
duration, and a declared residue. These are demonstration tuning, not a new
engine-wide combustion law. The demo supplies combustible Material; the
engine does not reinterpret every wood-colored Mod Material as flammable.

Render a basic visible fire indicator and emitted light. A flame overlay must
not replace its supporting solid, alter collision, or require a Look pack.
Smoke simulation, particle polish, oxygen, weather, conductive thermodynamics,
temperature-driven phase transitions, health/damage, and entity burning are
not included by this local World-Material model.

### 11. Lighting, meshes, and responsiveness

Optical interpretation combines the actual solid, fluid, and burning state.
Keep the current two 0-15 light channels. Fluid dimming and emission remain
dimensionless attenuation/emission values; burning emission disappears when
extinguished or consumed. Test both light increases and decreases.

Retain one shared light value per Voxel. Disconnected voids in one Voxel may
share brightness; precise per-void lighting is outside this phase. Keep solid
face masks and joint-seal rules. Combine independent emission by maximum and
combine translucent dimming by maximum rather than charging twice for one
Voxel. Document this approximation in the fresh-solve oracle as well as the
incremental path; neither thermal nor fluid passage may use it as geometry.

Do not route fluid changes through the existing upward-only streaming solve
as though it were a general bulk Edit relight. Preserve correctness across
neighbor Chunks and reject solves that read obsolete simulation state.

Reusing incremental relight is the initial implementation path. A general
bulk relight rewrite remains a named follow-up. Measure its relevance to the
accepted fluid/fire scenarios before choosing final budgets. If the phase
cannot meet its responsiveness requirements without bulk relight, report the
evidence and propose an explicit prerequisite/scope revision. Do not silently
add that rewrite or mark the phase complete with unresponsive flow.

Coalesce mesh invalidation per changed Chunk and boundary. Geometry, light,
and auxiliary state changes all invalidate the appropriate Snapshots. Retain
the existing progress principle that an older completed mesh may be delivered
while a newer one is queued; continuously changing water or fire must not
starve mesh delivery forever.

## Testing Decisions

Prefer the highest existing seams: the native simulation interface for exact
state evolution, the Server's controlled-tick harness for authority/streaming/
light/mesh behavior, and independent WASI guests for declarations and Commands.
Extend the existing held/scrambled Job tests rather than creating a second
fake scheduler. Use live captures for visible geometry and controls, not for
proving deterministic state.

Acceptance criteria:

1. Breach a generated lake into a dry Cave: it produces downward flow and
   bounded shallow spread, without automatic sources or upward filling.
2. Across 1.0, 0.5, and 0.25 m Voxel sizes, equal physical fixtures respect
   metre-valued reach/rates and reset-drop behavior within the stated
   spatial/time quantization, including a half-metre drop inside a Shape.
3. Cutting supply or removing a source removes unsupported flow, including
   loops; replacing a source or fuel while a Job runs cannot resurrect it.
4. Flow crosses every loaded Chunk face, including negative coordinates.
   At missing neighbors it pauses; load wakes it, eviction clears its lifetime,
   and generation order does not leave a permanent seam.
5. Waterlogging preserves slab/stair/post/fence geometry and collision at all
   relevant Orientations and connection patterns. Joint seals, internal
   partitions, and disconnected voids prevent leakage and false reactions.
6. Fluid placement/removal can target waterlogged state without deleting its
   solid. Failed edits are atomic and still enforce origin, reach, and caps.
7. A real Mod declares lava and another fluid with different metre-valued
   behavior. Invalid declarations/references are refused before World open.
8. Existing 0.1 fixtures still load; a 0.2 fixture exercises fluid declarations,
   batched source control, and typed player Edit history without host imports
   that reach a Voxel directly.
9. Water/lava and a custom pair react deterministically. Reactions preserve
   occupied Shapes and either apply the complete result or consume neither
   input. Unmatched fluids do not mix or overwrite sources.
10. Exposed combustible Material ignites after its delay, spreads fire,
    consumes fuel, leaves its declared residue, and eventually settles when
    fuel is exhausted. A solid wall blocks exposure; an open path can admit it.
11. Waterlogging extinguishes and protects the whole solid Voxel, including a
    dry part of a partly wet Shape. Dry fuel burns as a whole Shape; solid
    residue preserves its geometry. Removing water permits ignition after a
    fresh delay. No thermal state appears in distant, cold, inactive terrain.
12. Light equals an independent fresh solve of the final cell state after
    adding/removing water, lava, fire, residue, and waterlogged geometry,
    including cross-Chunk cases and stale light Jobs.
13. Full-detail and held level-1 meshes are redelivered after fluid/fire
    changes. Generated-waterline correction does not move waterfalls or
    waterlogged surfaces. Coarse ring masking retains its existing coverage
    guarantees; higher-level limitations are documented.
14. Replaying identical step-indexed events under different hash seeds,
    budgets, worker counts, and scrambled completion produces equal canonical
    simulation state. Test light/mesh settling separately from simulation time.
15. Evaluations, mutations, activation scans, queue growth, retained results,
    and in-flight work obey configured count bounds. Saturation preserves
    progress and fairness for player Edits, Mod Commands, fluids, and burning.
16. Dry/unburning Chunks and uniform lakes retain compact storage. After
    settling, idle terrain performs no recurring per-Voxel simulation work.
17. Live captures show a waterfall, waterlogged geometry, emissive lava, a
    burning combustible fixture, extinguishing, and subsequent mesh updates.
    Placeholder appearance is sufficient; no water shader polish is required.
18. Benchmarks and full CI pass the verification policy below. Any remaining
    bulk-relight prerequisite is surfaced before implementation is called done.

### Measurement and verification policy

Add native fluid/thermal benchmarks and retain existing meshing, collision,
and LOD comparisons where changed. Measure inactive terrain, a finite active
waterfall, waterlogging-heavy geometry, reactions, fire spread, and a saturated
frontier. Record native work/CPU, memory, backlog, logical progress, landed
changes, mesh progress, and complete Server tick distributions.

Use the same physical fixtures at 1.0, 0.5, and 0.25 m Voxel sizes. Compare
equal simulated work as well as observed live behavior; process CPU during
unequal presentation progress is not throughput. Before/after CPU comparisons
must be interleaved, with uptime before and after every process and load beside
every figure. Keep unique temporary paths. Retain raw outliers, identify
discarded distorted pairs, and do not select defaults from distorted loads.

Use the existing 64 Hz Server tick's 15.625 ms interval as the responsiveness
reference. Choose operation caps from repeated quiet-load measurements that
leave room for existing streaming and Edits. Report baseline/control and
fluid/fire tick distributions, active-frontier progress, and source-to-visible
latency together. A quota proves bounded work, not a universal wall-time bound;
neither tiny quotas with unusable flow nor an unqualified timing promise are
acceptable substitutes for that evidence. Numerical performance budgets and
representative fixture sizes are finalized in the measurement ticket before
claiming phase completion.

Before each commit run cargo fmt --all, npm run fmt, and the complete
scripts/cargo-ci workflow. Do not skip the independent guests, default build,
boundaries, advisory checks, or bench compilation when available. Preserve
Wasmtime/WASI 49.0.4 and Cranelift 0.136.4 fixes.

Hand-wrap this directory's Markdown at 80 columns. Prettier ignores .scratch;
never run it directly here. Finished implementation tickets set Status: done
and append outcome, verification, and any measurement evidence under Comments
in the same commit that completes the ticket. Use Conventional Commits with
the requested Co-Authored-By trailer.

## Out of Scope

- Water shading, Look packs, reflections/refraction, and visual polish: step 9.
- Conserved-volume liquid, pressure equalization, or automatic source creation.
- Simulation of unloaded terrain, terrain loading driven by flow, offline
  catch-up, save/load implementation, and simulation at coarse LOD levels.
- Gas/smoke transport, conductive temperature fields, melting/boiling/freezing,
  regional fuel destruction, per-void light storage, weather, oxygen/chemistry,
  entity health or burning, inventories, and recipes.
- A general bulk relight rewrite without an explicit, evidence-backed scope
  revision. The existing two-Voxel Mod drain cap is not raised implicitly.
- The known fbm2 debug overflow for seeds within 8 of u32::MAX.
- Changes to the main checkout or phase 7 worktree, merging phase 7, pushing,
  or opening a phase 8 PR during this planning authorization.

## Further Notes

Planning started from local phase 7 commit
a60d1f998a921898df72eb4bc71c98f536937c2a on the explicitly bound
feat/phase-8-fluid-simulation worktree. Phase 7 PR 18 was squash-merged to
main as ee77e54, and on 2026-10-22 this branch was rebased onto that commit,
so main is its direct ancestor and no phase 7 change is duplicated.

Relevant ADRs: 0001 (native algorithms and batched WASM), 0002 (Server
authority), 0003 (per-World Voxel size), 0006/0009 (table identities), 0007
(Snapshot Jobs), 0010 (derived connections), 0011 (derived coarse levels), and
0012 (declaration and Command lifecycle). The representation and new contract
will need focused ADR/glossary updates when accepted; no existing ADR is
silently superseded by this draft.

See investigation.md for the initial code findings and thermal-prior-art.md
for the Minecraft/Luanti comparison. Neither reference promises that those
games universally block ignition through walls. Solid-blocked local exposure
is the behavior explicitly selected for Tesserae.

## Comments

- Draft synthesized from the user's approved behavior and code investigation.
  Engineering recommendations are called out separately for document review.
- Builder review identified four missing policy details. Proposed resolutions
  are whole-Voxel wet-fuel protection/consumption, physical reset-drop distance,
  shared per-Voxel light, and blocking between unmatched fluid Materials.
  These are engineering recommendations awaiting document approval.
- 2026-10-22: reviewed as written; no ticket was merged or split. Every ticket
  is ready-for-agent and 01 is the only unblocked one.
- 2026-10-23: the user approved an amendment to decision 10. Fuel was wet only
  when an extinguishing fluid stood in its own voids, and a full cube has none,
  so water could never put out or protect the most common fuel, defeating user
  story 17. In Minecraft and Luanti fire lives in the open cell beside its fuel,
  so water flowing into that cell puts it out. Fluid in a neighboring void that
  reaches the fuel's face now wets it too; the rest of the whole-Voxel rule and
  its geometry are unchanged.
