# Fluid and fire live beside the Voxel, and one reading interprets both

Status: accepted.

## Context

A Voxel is a packed Material, Shape and Orientation with permanent ids
([ADR-0006](0006-shapes-are-table-data.md),
[ADR-0009](0009-materials-are-table-data.md)). Step 8 needs flowing water with a
surface, reach and descent, water standing inside a stair, lava a Mod declares,
and burning fuel with fuel left. None of it fits in those bits without encoding
flow in Orientation, reserving Shape ids, widening every Voxel or renumbering
Materials, and most terrain holds none of it. There is no save yet, so nothing
durable says what survives eviction.

## Decision

**Fluid and fire are a second layer of each Chunk, the Overlay, beside the
packed Voxel.** It is palette-compressed like the Chunk, so a Chunk that holds
no fluid and no fire stores one uniform entry and nothing dense, and generated
lake water stays a packed water cube with an empty Overlay entry. Every Voxel
has exactly one interpretation, its Occupants (`Occupants::of(voxel, overlay)`
and its inverse `encode`): a packed fluid cube is a full source with no solid;
every other wet state, flowing, a partial surface or fluid inside a Shape, lives
in the Overlay with its Material. One fluid has one home, never two copies.
Generation, legacy Commands, picking, light, meshing, LOD and the simulation all
read Occupants, and every writer goes through one World change, `set_occupants`,
which vets the reading and keeps the encoding canonical.

**One fluid Material per Voxel, separate state per Void.** A Voxel holds at most
one fluid Material. A Shape's Voids are derived at registration from its
sixteenth grid, Orientation and arms, and each wet Void keeps its own surface,
reach and descent (`Fluid` and `Fluid::others`), because one shared state would
let a wet Void lend reach across a partition. Different fluids never share a
Voxel, and fluids meet only through Voids whose open squares meet across a face.

**Light is one value per Voxel.** A wet or burning Voxel is lit as a whole: the
solid's seals, the larger of the solid's and the fluid's dimming, the brighter
of their emissions and the fire's (`Tables::wet_light_params`). The incremental
relight and the fresh solve (`light_cells`) use the same approximation, so they
stay equal. Disconnected Voids of one Voxel share brightness. Neither flow nor
heat reads light as geometry.

**Fuel burns, and is wet, as a whole Voxel.** A Shape does not split into
separately consumable parts. An extinguishing fluid touching any part of the
fuel, in its own Voids or from a neighbour's Void across a face its solid
covers, protects all of it and puts out its fire. Burning is a fuel count on the
Voxel; burning out turns the solid into its residue in the same Shape and
Orientation. Exposure is computed when fuel is evaluated and never stored, so
cold terrain carries no thermal state at all.

**Durable versus derived state.** No save exists yet. What a later save must
store to resume a World exactly, and what it may rebuild:

| State                                                                                             | Kind    | Where it lives now                                   |
| ------------------------------------------------------------------------------------------------- | ------- | ---------------------------------------------------- |
| Packed Voxels: Material, Shape, Orientation                                                       | durable | `Chunk`                                              |
| Fluid per Voxel: Material, source or flowing, and per Void surface, reach and descent             | durable | `ChunkOverlay` (`Overlay::fluid`)                    |
| Explicit source identity: a packed fluid cube, or `Fluid::source`                                 | durable | `Chunk` and `ChunkOverlay`                           |
| Burning and its fuel left, in steps                                                               | durable | `ChunkOverlay` (`Overlay::burning`)                  |
| Running flow timers: the change a Voxel waits to make and the step it may                         | durable | `flow::Timers`, held by the Server's `FluidSim`      |
| Running fire timers: ignition progress and burn-out, running or frozen beside unheld terrain      | durable | `flow::Timers` (`fire`)                              |
| Pending wakes and Rescans, with their cursors                                                     | durable | `agenda::Agenda`                                     |
| The Logical step the simulation is at                                                             | durable | `FluidSim`                                           |
| Content identity: Material and Shape names in Registry order, fluid, reaction and combustion rows | durable | the Mods' Declarations, replayed at load             |
| Voxel size, seed, World parameters                                                                | durable | `WorldSettings`, the Game's Declaration              |
| Light, meshes, level-1 and coarser Chunks, connections                                            | derived | rebuilt from the above                               |
| Snapshots, Jobs in flight, landing cursors, the change journal, Lifetimes, spoilt-solve counts    | derived | rebuilt or reset                                     |
| Exposure, wetness, the heat and flow scales                                                       | derived | recomputed from Occupants, tables and the Voxel size |

A timer's step is absolute, so a save stores it relative to the Logical step.
Without the timers, Agenda and step, a reload still settles, but not to the
state an uninterrupted run reaches at the same step.

**Today eviction discards.** Evicting a Chunk drops its Overlay, its timers and
its pending work with its solid Edits; regenerating it reproduces its generated
sources and nothing else, and its new Lifetime refuses work read from the old.
Held neighbours re-evaluate: flow beside a gap pauses, neither spreading into it
nor receding from it, and drops any change it was waiting to make; fuel beside a
gap pauses with its ignition progress and fuel left frozen and goes on once the
gap is held again, unless its solid was replaced meanwhile.

## Consequences

- A dry, unburning Chunk costs one palette entry and one `Arc` more than before;
  a uniform lake costs nothing more. One wet Voxel promotes its Chunk's Overlay
  to a dense index, 4 KiB plus palette entries of 30 bytes.
- Everything that reads cells must read Occupants. A second reading of water
  anywhere would be two mutable copies and is the mistake to look for.
- Per-Void light is out of scope. A fluid that is opaque and emits nothing draws
  its partial surfaces, and the Shapes it fills, with its own dark light; lava
  emits, water and the demo's mud are translucent. Precise per-Void light is a
  later decision, not an extension of this one.
- A Shape with more than four Voids under any arms (`MAX_VOIDS`) holds no fluid.
- Burning fuel cannot be partly consumed, and a thin film of water against a
  cube wets all of it; both follow from the whole-Voxel rule.
- The save step inherits the inventory above, including the timers and the
  Logical step.
