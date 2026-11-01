# Simulation advances in logical steps under count bounds

Status: accepted.

## Context

Fluid, reactions and fire change World state on their own, across Chunk faces,
while players edit, Mods drain their queues and streaming admits and evicts
Chunks. Work runs on workers that finish in any order
([ADR-0007](0007-streaming-jobs-run-on-bevy-task-pools.md)), only the Server may
change the World ([ADR-0002](0002-client-server-from-day-one.md)), and the
expensive part must stay native and headless
([ADR-0001](0001-wasm-mod-boundary.md)). A waterfall must look the same at any
Voxel size ([ADR-0003](0003-voxel-size-is-per-world.md)), and no amount of water
may take a tick from the player.

## Decision

**One native kernel, one interface.**
`flow::advance(view, tables, scale, continuation, quota)` reads immutable cells,
never writes, and returns either a continuation or a finished step: change
proposals, each with the Occupants it read, wakeups, timers and work counts.
Flow, contact reactions and fire are evaluated by it for the same due Voxels.
Tests, benches and the Server's worker Jobs call the same function; nothing
re-implements Shape passage, burning or wetness.

**Time is logical.** A Logical step is 1/64 s. Every rate and delay in metres
and seconds becomes whole steps when the World opens (`FlowScale`, `HeatScale`,
`Pace`), measured from the World origin so a straight path is within one step of
its exact time at every Voxel size. An event, a player Edit, a drained Mod
Command, a Chunk's admission or eviction, takes the index of the step under way
and is first read by the next. A step's Snapshot is taken when it starts and it
lands after every event of its index, so equal state and equal ordered events
give equal state at every step, whatever the worker count, the scheduling or the
hash seeds. It promises nothing about frames or wall-clock ticks.

**Landing is the Server's, through the one World change.** Each proposal lands
through `set_occupants`, with Edit validation, shaped caps, relight and mesh,
boundary and level-1 invalidation. It is dropped and its Voxel woken if any
event of its step touched what it read, if a Chunk it read changed Lifetime, or
if its Voxel no longer reads what it was computed from. A reaction writes one
Voxel and depends on two, and is revalidated against the step's earlier
landings, so it applies whole or not at all.

**Two kinds of knob.** How a step is divided, evaluations per Job and Jobs in
flight, changes nothing a step computes. The Agenda's capacity, the Rescan
budget, `LANDED_PER_STEP` and `EMITTERS_PER_STEP` are rules of the simulation:
under load they decide, deterministically, when a Voxel changes, never whether.
An Agenda that is full turns wakes into Rescans of their Chunk instead of
dropping them; a change past a landing bound is deferred with its timers held,
so it lands the next step rather than starting its delay again. Landing past a
bound goes one change per Chunk in turn from a cursor, and the emitter bound
keeps a step's relight work to a few emitter changes, so water is never held
behind fire. Bounds count operations, not time.

**Quiet terrain is free.** Nothing is evaluated until something wakes it. A dry
Chunk is admitted with no scan; a Chunk with fluid is scanned once on admission;
settled flow and cold fuel keep no timer and no Agenda entry. Unknown terrain is
never Air: work beside it pauses until admission.

**Light keeps up with the simulation.** A light solve that read a plane a change
moved is dropped; after four in a row the next is made on the tick
(`SPOILT_BEFORE_ON_TICK`), so a writer changing a boundary every tick delays a
neighbour's light, and the meshes waiting on it, by at most four solves.

## Consequences

- Equal bounds are part of an equal setup: two Worlds with different landing
  bounds are different simulations, not two schedules of one.
- A quota bounds work, not wall time. The measured budgets and their caveats are
  in [DESIGN.md](../../DESIGN.md#road-to-version-10), step 8.
- Every new kind of simulation joins the kernel, the Agenda and the landing
  rules; a second scheduler or a second write path is the mistake to look for.
- Player Edits and the fair Mod queues run before the step each tick and are
  never deferred by it; the phase 7 drain cap of two Voxels a tick is unchanged.
- A general bulk relight remains a follow-up. It would cut the costliest case
  measured, lava draining outdoors at fine Voxel sizes, and let the emitter
  bound rise.
