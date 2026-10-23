# 07: Ignite, burn and extinguish declared fuel

Status: done
Blocked by: 04, 05

Spec: [decisions 3, 6, 10 and 11](../spec.md).

## What to build

Hot lava and burning Material expose nearby reachable fuel to heat. Fuel
ignites after its declared delay, spreads fire, burns to its declared residue,
and can be extinguished and protected by waterlogging.

## Acceptance criteria

- [x] Native evaluation uses declared heat strength/reach, ignition threshold
  and delay, finite burn duration, emission, and residue. Spatial units are
  metres and elapsed time is logical simulation seconds.
- [x] Heat follows reachable open space and attenuates monotonically over
  physical path length. Solid walls, Shape partitions, joint seals, and missing
  Chunks block it; an open route within reach can transmit exposure.
- [x] The strongest source determines exposure. Dropping below the threshold
  resets incomplete ignition progress. No temperature field or solid heat
  conduction is introduced.
- [x] Fuel is whole-Voxel solid state. Waterlogging by an extinguishing fluid
  anywhere in accessible voids protects the whole fuel, extinguishes burning,
  and resets ignition; it does not leak suppression through neighboring walls.
- [x] Dry fuel ignites, spreads exposure, consumes finite fuel, and replaces
  its solid Material with residue while preserving Shape/Orientation, or Air.
  Replacing/removing fuel ends obsolete burning.
- [x] Residue only burns under its own declaration. Ordinary fire eventually
  settles without fuel; permanent sources require explicit declaration.
- [x] Removing water allows reignition after a fresh normal delay. Different
  declared fluids can have different extinguishing behavior.
- [x] A basic flame indicator and block light reveal burning without replacing
  collision geometry or depending on a Look pack.
- [x] Shared quotas/fairness also cover thermal work. Stale Jobs, step replay,
  Chunk load/eviction, and saturated frontiers cannot bypass the contracts
  established for fluids.
- [x] Tests cover partial wet Shapes, protected fuel, finite fuel chains,
  source removal during Jobs, light decrease on extinction, and controlled
  native/Server/real-Mod behavior.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- This delivers the approved local heat/fire model. Regional fuel destruction,
  conductive temperatures, gases, weather, and entity damage are outside it.
- 2026-10-23: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- `tesserae-core/src/fire.rs` (new), evaluated by `flow::advance` for the
  same due Voxels, quota, timers and wakeups as flow. `HeatScale` (inside
  `FlowScale`, now `FlowScale::new(&Tables, voxel_m)`) tabulates each hot
  fluid's and each fuel's heat at World open, the ignition delays in steps,
  and `radius()`, how far a fuel's evaluation reads.
- Exposure: a breadth-first search of open space (the `Void`s flow uses,
  fluid-filled or not) from the space touching the fuel (its own voids and
  the voids across its faces that meet its surface). Heat crosses between
  voids only where their open squares meet, so walls, partitions, sealed and
  jointly sealed faces pass nothing; unknown terrain is a wall. Sources: a
  hot fluid's wet voids, a burning solid's voids, and voids meeting a
  burning solid's surface. Path length is face steps times the Voxel size:
  `n` Voxels of gap is `n` steps from a hot fluid and `n - 1` from a burning
  surface (one Voxel of resolution). Attenuation `floor(strength * (reach -
  k v) / reach)`, 0 from `reach` on, never increasing; the strongest source
  wins (max, never a sum). The search stops at the deepest step that can
  still reach the lowest declared threshold, or as soon as nothing farther
  could beat what it found. No temperature is stored anywhere.
- Ignition and burning are timers (`Timers` gained a `fire` map beside
  flow's, split, merged and retained with it): exposure at or above the
  threshold starts the delay (rounded steps, at least one); any evaluation
  below it, or wet, drops it, so progress resets. When it runs out the fuel
  lands `Burning { fuel: fuel_steps }`; a burning Voxel's timer runs that
  fuel down and then `place_solid`s the residue with its Shape and
  Orientation (air removes the solid, fluid in its voids is carried as a
  placement carries it). `Burn::Permanent` holds no timer.
- Wet (`fire::is_wet`, also used by the player/Mod `ignite`): an
  extinguishing fluid in any of the Voxel's own voids protects the whole
  fuel and puts its fire out the next step, no residue.
- One `Proposal` per Voxel per step: flow's fluid change and the fire
  change combined. `Proposal.heated` marks an ignition, decided from
  exposure.
- `Tables::vet_occupants` refuses burning on a solid with no combustion row
  (`PlaceFault::NotFuel`, `RejectReason::NotFlammable`); the ticket 01 to 04
  tests that stored fire on stone now declare stone fuel (`stone_burns`) or
  use declared fuel. `Tables::burning_emission`, `CombustionTable::is_empty`,
  `FaceMask::complement`, `fire::alters_exposure`, `wake_fuel_around`,
  `fuel_spans_around`, `exposure_at`. `Work` gained `fuel` and `searched`.
- Light: `LightVolume::burning` (default false) and the bulk `light_cells`
  sweep add a burning solid's declared emission by the larger, the same
  approximation both ways; `set_occupants` relights when burning changes.
- Mesh: pass 5 draws a flame quad a sixteenth outside every face of a
  burning solid that the Voxel across does not cover, in the cutout layer
  with `mesh::FLAME` (past every Material id). The client palette has one
  slot more for it (`FLAME_SLOT`), chosen by the shader, so no appearance
  row or Look pack can recolour it. Collision and selection read the Voxel
  only.
- Server (`fluids.rs`): the journal marks changes that can alter exposure
  (`Changed.heated`: a solid, a fire, a hot fluid); these wake the fuel
  within `radius()` in Chunks whose palette holds fuel, on events and on
  landing. A step snapshots every Chunk within the radius of each due fuel.
  An ignition is stale if any event of its step touched anything within the
  radius or any Chunk there changed state. Admission, eviction and a
  journal-coalesced Chunk rescan the fuel within reach of that Chunk
  (budgeted rescans). Agenda, `Limits`, landing bound, parking, the event
  table and Lifetime staleness are flow's, unchanged.
- Demo (0.2): `demo:wood` (10) and `demo:ash` (11) with the spec tuning:
  threshold 6, 2 s delay, emission 12, heat 12 over 2 m, 8 s to ash. Its
  build is unchanged. `docs/architecture.md` lists `fire.rs`.

### Verification

- Test functions: core lib 309 (292), meshing 64 (63), Mods 89 unchanged
  (three ignored as before), `tesserae` binary 331 (324, one ignored as
  before); workspace 793 (768). Full `scripts/cargo-ci` passed on the tree
  this commit records.
- Kernel (`flow::tests::fire`, three sizes where physical): wood two Voxels
  from a permanent fire catches exactly 128 steps (2 s) after it is placed,
  burns exactly 512 (8 s), becomes ash in the same Shape and stays ash, the
  permanent fire keeps no timer; a turned stair keeps Shape/Orientation in
  its residue and residue air removes it; a finite chain (cube, gap, cube)
  catches 128 steps apart, burns out and settles with no timer, to ash or
  air; the tables and the search give `12 * (2 - d) / 2` over metres of path
  from lava and from a fire, never rising, nothing past reach; two sources
  of 6 give 6; a wall blocks, a two-step detour admits (7, catches), a
  four-step one does not, a partition and a top/bottom slab pair block, two
  bottom slabs pass; hidden (unknown) terrain blocks; a wood wall Shape wet
  on the far side of its partition never catches though its dry side faces
  the fire, catches dry, and is put out within steps of water arriving;
  water in the next Voxel does not protect a cube; waterlogging mid-delay
  resets and drying gives exactly a fresh delay, as does a fire going out
  and relit; stone replacing burning wood never becomes ash, fresh wood
  replacing it takes fresh timers; wood to charcoal to ash beside a
  permanent fire, ash never burning; lava flowing to 3 m lights wood a
  Voxel past its front; a waterlogging fluid that does not extinguish does
  not protect, one that does does; the read radius follows the lowest
  threshold (1, 2, 4 steps at 1, 0.5, 0.25 m, plus 3) and is 0 with no
  fuel; quotas 1, 3, 17 and 2, 3, 7 slices finishing last-first give the
  same state at all 1600 steps and the same work; cold fuel settles with no
  timer and no work over 500 steps.
- Meshing: a burning cube draws five flame quads (not the face against
  stone) outside its Voxel, cutout, `FLAME`, and the same opaque geometry as
  unburnt.
- Server (`fluids::tests::fire`): a Mod's ignite lights the corridor to 11
  beside the wood, the next wood catches 128 to 130 steps later, both leave
  ash, the light goes to 0, each time equal to a fresh `light_cells`, no
  timer left, at 1.0, 0.5 and 0.25 m; the fluid tool waterlogging a burning
  stair puts it out within 3 steps, its light falls (fresh-solve equal),
  and removing the source relights it a fresh 128 to 130 steps later; a
  held step due to ignite lands nothing after the player breaks the fire
  (stale rises), and a held step due to burn out lands no ash after stone
  replaces the wood; division 2 x 1 and 3 x 8 scrambled against the default
  give equal state at every step and equal work through a chain, a stair
  and water; evicting the fire's Chunk drops the wood's progress within four
  ticks, it never catches while absent, the regenerated Chunk holds no fire
  and relit it catches a fresh delay later; under an agenda of 32 + 16 and
  one landing a step, with two Mod queues and a player Edit, both bounds
  bite, the queues keep one Voxel a tick each, the Edit lands on its tick
  and the corridor settles equal to the unbounded run; real WASI guests:
  demo lava ignites demo wood a Voxel past its 3 m front (after its arrival
  plus the delay) which burns to `demo:ash`, the `fluids` guest's slime
  waterlogging its wood stair keeps it from catching while dry it catches,
  and its wood lit by a 0.2 `ignite` burns 512 to 514 steps to its ash.
- Mutation checks, each failing the tests named: passage ignoring open
  squares (walls test); progress kept below the threshold (reset test);
  never wet (partial-wet, reset, per-fluid tests); summing sources
  (strongest test); no ignition staleness (older job); landing waking no
  fuel (real-Mod test); relight ignoring fire, and the bulk solve ignoring
  fire (both Server light tests); eviction rescanning no fuel (eviction
  test); a step not snapshotting the heat radius (real-Mod test).
- A live capture of the dev build rendered normally with the new shader
  branch (validated, deleted after review); it shows no fire, the demo
  builds none.
- No Client mutator; core gained no dependency; `Cargo.lock` unchanged.

### Deviations, limits and what later tickets inherit

- A full cube has no void, so water can never wet it: water protects and
  puts out Shapes; a burning cube burns out or is put out by a Mod's
  `extinguish`. Water next door does not protect, per the spec's
  whole-Voxel rule, and heat passes through fluid-filled space, so only
  water in the fuel's own voids suppresses.
- Putting a fire out discards its spent fuel: relit, it has its whole fuel.
- Exposure below every fuel's threshold reads 0 (`exposure_at`): the search
  stops where nothing could catch.
- A Mod extinguishing and relighting one Voxel within the same step can
  keep the old burn-out timer (flow's same-want timer rule); not otherwise
  reachable.
- For ticket 09: unmeasured: fire timers hold an agenda entry for their whole
  delay or burn, one per igniting or burning Voxel, so a fire larger than
  the agenda capacity slows other work through rescans (a 16 + 8 agenda
  with four fires starved a stream in testing; 32 + 16 was needed). A Chunk
  whose palette holds fuel is rescanned whole on admission (32768
  evaluations). Each fuel evaluation searches up to (radius)^3 voids; heat-
  relevant changes scan a radius box in fuel Chunks. No bench was rerun.
- For ticket 08: level 1 does not show fire; flames and fire light are
  level 0 only.
- For ticket 10: exposure, ignition progress, burn-out, flame indicator and
  wet fuel are new nouns for CONTEXT.md; the durable inventory should count
  fire timers (ignition progress, fuel left) with flow's. No live capture of
  a burning fixture yet.
