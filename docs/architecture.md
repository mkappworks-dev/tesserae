# Architecture

Three crates. The boundary that matters: `tesserae-core` has no Bevy and no
wgpu, so every expensive algorithm is testable and benchmarkable headlessly.

```mermaid
flowchart TB
    subgraph SANDBOX["WASM sandbox — untrusted"]
        GAME["Game<br/><i>mod with root manifest</i>"]
        MOD["Content mods<br/><i>blocks, entities, recipes</i>"]
        LOOK["Look packs<br/><i>WGSL + post settings</i>"]
    end

    subgraph MODS["tesserae-mods — ours · no Bevy"]
        WIT["WIT contract<br/><i>versioned</i>"]
        HOST["wasmtime host"]
        VALIDATE["Registry + Command validation"]
    end

    subgraph APP["tesserae — ours · the Bevy app"]
        subgraph SRV["Server — sole world authority"]
            TICK["Tick loop"]
            MODJOBS["Owned asynchronous Mod Jobs"]
            QUEUE["Fair pending Command queues"]
            STATE["World state"]
            STREAM["Chunk streaming<br/>+ LOD rings<br/><i>coarse levels derived,<br/>never stored</i>"]
            JOBS["Jobs on the async<br/>compute pool"]
        end
        subgraph CLI["Client — never mutates state"]
            INTENT["Camera · input · UI"]
            UPLOAD["Mesh upload → Mesh3d"]
            MATEXT["MaterialExtension<br/><i>chunk · water · foliage</i>"]
        end
    end

    subgraph CORE["tesserae-core — ours · no Bevy · no wgpu"]
        STORE["Chunk storage + palette"]
        GEN["World generation"]
        MESHER["Greedy mesher<br/><i>2-pass, shaped voxels</i>"]
        LIGHT["Light flood-fill<br/><i>incremental</i>"]
        FLUID["Fluid simulation"]
        COLL["AABB vs voxel collision"]
    end

    subgraph BEVY["Bevy 0.20 — third party"]
        ECS["bevy_ecs · app · asset<br/>winit · audio · input"]
        PBR["bevy_pbr<br/><i>shadows · SSAO · TAA<br/>bloom · volumetrics · tonemapping</i>"]
    end

    GPU["wgpu 30 → Metal · Vulkan · DX12"]

    GAME --> WIT
    MOD --> WIT
    LOOK --> WIT
    WIT --> HOST --> VALIDATE
    HOST --> MODJOBS --> QUEUE --> TICK
    VALIDATE --> QUEUE

    INTENT -- intent only --> TICK
    TICK --> STATE --> STREAM
    STREAM -- "Arc snapshots in,<br/>owned results out" --> JOBS
    JOBS --> GEN
    JOBS --> LIGHT
    JOBS --> FLUID
    TICK --> COLL
    STATE --> STORE
    JOBS --> MESHER
    STREAM -- "landed on the tick" --> UPLOAD

    LOOK -. WGSL asset .-> MATEXT
    UPLOAD --> PBR
    MATEXT --> PBR
    APP --> ECS
    PBR --> GPU
    ECS --> GPU

    classDef ours fill:#1f6feb22,stroke:#1f6feb,stroke-width:2px
    classDef third fill:#8b949e22,stroke:#8b949e,stroke-width:1px,stroke-dasharray:4 3
    classDef untrusted fill:#d2992222,stroke:#d29922,stroke-width:2px
    class MODS,APP,CORE,SRV,CLI ours
    class BEVY,GPU third
    class SANDBOX untrusted
```

## Where everything sits

Orange is untrusted and sandboxed. Blue is ours. Dashed grey is third party.

| Question                      | Answer                                                                                                                                                          |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Where is the engine?**      | All three crates. `tesserae-core` + `tesserae-mods` + `tesserae` _is_ the engine.                                                                               |
| **Where is the game?**        | Outside the repo. A Game is a WASM Mod with a root manifest — orange box, top. One loader for Games and Mods both.                                              |
| **Where is Bevy?**            | Dashed grey, bottom. Only `tesserae` touches it: ECS, app, assets, windowing, input, audio, and `bevy_pbr` for all lighting.                                    |
| **Where is our custom code?** | Blue. `tesserae-core` holds every expensive algorithm with no edge to Bevy or wgpu — that is what makes it benchmarkable. `tesserae` is thin glue.              |
| **Where is the renderer?**    | There isn't one. `bevy_pbr` rasterizes; we supply meshes and `MaterialExtension` WGSL. The decision that saves 12–24 months.                                    |
| **How do Mods work?**         | WASM → WIT contract → asynchronous Wasmtime call → validation → Server-owned **batched** queue → Server tick. Never a direct call into core.                    |
| **How do Look packs work?**   | WASM cannot put code on the GPU. The pack ships WGSL as an _asset_ and registers it by handle; the host compiles it into a `MaterialExtension`. Hot-reloadable. |

## Mechanics are the engine's, content is the Game's

The engine owns how things behave; a Mod's Declaration says which Materials
behave that way, and with what numbers. The engine has no water, lava, wood or
mud; those are the demo Game's. Its only built-in Material is air. The Game
names the rock, soil and surface of its ground and the fluid filling its lakes
([ADR-0016](adr/0016-only-air-is-built-in.md)).

| Behaviour              | The engine decides                                                      | Data that switches it on, and who declares it                    |
| ---------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------- |
| A fluid flows          | Spreading, falling, receding and waterlogging, in metres and seconds    | A fluid row, from a Mod: the demo's water, lava and mud          |
| A fluid puts out fire  | How touching fuel wets it, puts its fire out and keeps it from igniting | `extinguishes` on the fluid row, from a Mod: the demo's water    |
| Lakes stand in valleys | Filling open air below a level with packed sources, in metres           | The Game's World parameters: a level and a declared fluid        |
| Two fluids react       | How a Contact is found and applied whole or not at all                  | A reaction row, from a Mod: the demo's lava and water make stone |
| Heat sets fuel alight  | Exposure through open space, ignition delay, burning and its residue    | A fluid's or a fuel's heat and a combustion row, from a Mod      |

A World with no Game is empty air: it has no ground, lakes, fluid or fuel. See
[ADR-0013](adr/0013-fluid-and-fire-live-beside-the-voxel.md) and
[ADR-0012](adr/0012-mods-declare-once-and-command-in-batches.md).

The Client sends aim reports beside edit intents. The Server answers with metre
outline bounds and an acceptable ghost cell through the edit resolver. The
Client meshes held Shapes from catalogue geometry and draws the outline, ghost
and corner model. The menu, hotbar and drag follower share a cached model atlas
from the same geometry. Games supply one optional, bounded texture per Material
through the Declaration's appearance rows. One GPU texture atlas is shared by
World surfaces and item models; the Client draws it without naming Materials.
Greedy terrain faces repeat per Voxel, and colour, depth and shadow passes use
the same texture alpha. Older contract lines retain colour fallback. Fire uses
engine flame artwork. The placement ghost uses the held model at 75 percent
opacity. The Client tracks a bounded history of aim requests and accepts
monotonic Server replies; it retains a compatible answer for at most 200 ms
while waiting for the next. Item changes, large turns or Eye displacement hide
it immediately. Sampling a Voxel is a separate read-only Server intent, not an
Edit; the Client applies a reply only if its hand selection has not changed. UI
atlas images use premultiplied blending to preserve transparency. No preview
reads World storage.

The Server also supplies removal target identity and bounds. The Client draws a
progress bar above it and sends a guarded removal after the Material's hold
time. This is a gesture guard, not a network anti-cheat timer. The unreleased
0.3 Material Declaration carries optional `remove-time-s`, validated from 0.05
to 60 seconds and defaulting to 350 ms, published in the content catalogue. Core
tables and simulation retain no input timing. The Server compares target
identity again before applying the removal.

Undo/redo owns at most 128 before/after Occupants entries and 16 queued requests
on the integrated Server. It checks Chunk Lifetime, settled light and exact
current Occupants before replaying through the normal change, light, mesh and
simulation wake path. Successful replay publishes inverse or forward Mod edit
events. New successful changes clear redo; refusals preserve history and World
state. Simulation consequences elsewhere are not rewound. Multiplayer history
ownership remains part of the network transport work.

## The three invariants this diagram exists to protect

Each of these is a mistake that will not look wrong in review.

1. **No arrow from Client to World state.** Only intent.
   ([ADR-0002](adr/0002-client-server-from-day-one.md))
2. **No arrow from Mods to core.** Only the batched queue.
   ([ADR-0001](adr/0001-wasm-mod-boundary.md))
3. **No arrow from core to Bevy or wgpu.** Ever.

## Layout

`▸ step N` refers to the build order in [DESIGN.md](../DESIGN.md).

```
tesserae/
├── CONTEXT.md                  glossary — the only names used anywhere
├── DESIGN.md                   decisions, build order, open items
├── Cargo.toml                  workspace
├── docs/
│   ├── architecture.md         this file
│   ├── positioning.md          vs Minecraft/Luanti — internal, not README
│   └── adr/                    0001 wasm · 0002 client-server · 0003 voxel size
│                               0004 demo game · 0005 asset root
│                               0006 shapes are table data
│                               0007 streaming jobs · 0008 player pose
│                               0009 materials are table data
│                               0010 connections are derived
│                               0011 coarse LOD levels are derived
│                               0012 Mods declare once, command in batches
│                               0013 fluid and fire beside the Voxel
│                               0014 bounded logical steps
│                               0015 water is Game content
├── crates/
│   ├── tesserae-core/          NO Bevy, NO wgpu — headless + benchmarkable
│   │   ├── src/
│   │   │   ├── voxel.rs        Voxel = material | shape | orientation       ▸ 1
│   │   │   ├── chunk.rs        palette-compressed 32³ storage               ▸ 1
│   │   │   ├── mesh.rs         mesher: pass 1 cubes, pass 2 shapes        ▸ 1, 5
│   │   │   ├── noise.rs        value noise 2D and 3D, and a range bound     ▸ 3
│   │   │   ├── worldgen.rs     seed + position → Chunk, tuned in metres     ▸ 3
│   │   │   ├── light.rs        incremental flood-fill                       ▸ 4
│   │   │   ├── shape.rs        Shape table, Orientation; derived geometry   ▸ 5
│   │   │   ├── lod.rs          downsample, and the covered-sky coarse light ▸ 6
│   │   │   ├── fluid.rs        fluid table, in metres and metres per second ▸ 8
│   │   │   ├── flow.rs         sources, flow and recession: the kernel      ▸ 8
│   │   │   ├── occupants.rs    fluid and fire beside the Voxel, one reading ▸ 8
│   │   │   ├── agenda.rs       bounded wakes; overflow rescans each Chunk  ▸ 8
│   │   │   ├── reaction.rs     fluid contact rules, canonical, as data      ▸ 8
│   │   │   ├── combustion.rs   heat, fuel and residue, as data              ▸ 8
│   │   │   ├── fire.rs         exposure, ignition, burning and residue      ▸ 8
│   │   │   └── collide.rs      AABB sweep and ray vs grid, in metres        ▸ 5
│   │   └── benches/        criterion — meshing.rs gates step 1          ▸ 1
│   │                       lod.rs: downsample, coarse Chunk, mesh, light  ▸ 6
│   │                       fluids.rs: flow, waterlogging, fire, rescans  ▸ 8
│   ├── tesserae-mods/          NO Bevy, NO wgpu — sandbox + validation     ▸ 7
│   │   ├── wit/0.1/            tesserae:mods@0.1.0, closed, world mod
│   │   ├── wit/0.2/            tesserae:mods@0.2.0: fluids, sources, fire  ▸ 8
│   │   ├── wit/0.3/            tesserae:mods@0.3.0: named lakes, fire Edits ▸ 8.1
│   │   ├── src/                loader, Trust, Registry, async calls, limits
│   │   │   ├── adapt/          each older line carried one line up and back ▸ 8.1
│   │   │   └── command.rs      pure batch validation, VoxelBox + BoxCursor
│   │   ├── tests/guests/       independent WASI sandbox fixtures
│   │   └── benches/            host crossings + native validation
│   └── tesserae/               the Bevy app
│       └── src/
│           ├── world.rs        Voxel size, seed, the Chunk lattice          ▸ 2
│           ├── server/         the sole authority over World state         ▸ 3
│           │   ├── world_state.rs  Chunk store — mutators Server-private    ▸ 3
│           │   ├── streaming.rs    rings, dispatch and landing              ▸ 3
│           │   ├── lod.rs          coarse levels: derived Chunks, LodStore  ▸ 6
│           │   ├── lod_rings.rs    one level's shells, with a hole inside   ▸ 6
│           │   ├── jobs.rs         jobs on the async compute pool           ▸ 3
│           │   ├── mods.rs         Mod Jobs, fair queues and tick drain      ▸ 7
│           │   ├── fluids.rs       flow on workers, one step at a time      ▸ 8
│           │   └── edit.rs         the Voxel-edit path, queued intents      ▸ 5
│           ├── client/         camera, input, UI, mesh upload               ▸ 2
│           │   ├── chunk_view.rs   one entity per Chunk the Server sent     ▸ 3
│           │   ├── edit.rs         place/remove intents, hotbar, crosshair   ▸ 5
│           │   └── menu.rs         tile menu and square play bar      ▸ 8
│           └── render/         chunk material, distance fog, look packs   ▸ 2, 9
│               ├── chunk_material.rs MaterialExtension and its WESL         ▸ 2
│               └── fog.rs            live FogSettings → one DistanceFog     ▸ 9
├── games/demo/                 MIT WASI guest, only the WIT contract       ▸ 7
├── assets/                     CC0 placeholder textures
├── LICENSE                     BSL 1.1 — blocked on the Change License check
└── USE-GRANT.md                the most important file in the repo
```
