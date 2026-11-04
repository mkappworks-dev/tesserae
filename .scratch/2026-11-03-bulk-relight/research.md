# Bulk Voxel writes and relighting: how other engines do it (research for Tesserae)

Date: 2026-11-03. Read-only research; no repo file edited except this one.

Question: today every Mod Command goes through the single-Voxel Edit path (`apply_intent` → `set_voxel` / `set_occupants`), each Voxel re-floods light and marks meshes stale, and the Mod drain is capped at `MOD_VOXELS_PER_TICK = 2`. How do other engines write a region or batch of Voxels and relight / re-mesh each touched Chunk once, and which approach fits Tesserae?

## Sources pinned

| Project | Source | Commit / version |
| --- | --- | --- |
| Tesserae | this worktree (`tech/bulk-relight`) | `ecb9689` |
| Luanti | github.com/luanti-org/luanti | `9c424c44ff523ebce076367e52222ac4d71e94c1` (2026-10-30) |
| Starlight (Fabric, MC 1.20.4, mod 1.1.3) | github.com/PaperMC/Starlight | `cca03d62da48e876ac79196bad16864e8a96bbeb` (2024-03-08) |
| Spottedleaf, "Starlight 1.20.md" (maintainer gist) | gist.github.com/Spottedleaf/6cc1acdd03a9b7ac34699bf5e8f1b85c | read 2026-11-03 |
| Paper (vanilla source patches, Mojang-mapped context lines) | github.com/PaperMC/Paper | `df120fd8b5211f61146a59f69497a5a62064b917` |
| FastAsyncWorldEdit | github.com/IntellectualSites/FastAsyncWorldEdit | `c4c57f22d192c8b4d4d38be78dc4aef57cf04927` (2026-11-02) |
| WorldEdit | github.com/EngineHub/WorldEdit | `130688b5d5705ff77d762b300ec8995414e1432c` (2026-11-03) |
| Mojang changelogs | minecraft.net snapshot posts 23w03a, 23w17a, 25w02a | as published |
| Vintage Story API (engine closed) | github.com/anegostudios/vsapi | `63d33f70018c606fb0f70ead900fe90625fbd7b2` |
| Vintage Story creative mod (WorldEdit) | github.com/anegostudios/vscreativemod | `1c649719734ac5c049ce947a945847b2a07e9f59` (2026-03-07) |
| Veloren | gitlab.com/veloren/veloren | `7daa95fd53ae62aa185d31ad316d0cabfe2d70ac` (2026-10-30) |
| Godot Voxel (Zylann) | github.com/Zylann/godot_voxel | `045ff9326a32fd10ac6fdac6357b5a098aaf2093` (2026-10-01) |
| Craft (Michael Fogleman) | github.com/fogleman/Craft | `d6888a6e1e54340358ce25b1220f410541360b6b` |
| Seed of Andromeda, "Fast Flood Fill Lighting in a Blocky Voxel Game: Pt 1" | archived copy, see `SOA1` | Wayback snapshot 2021-06-22 |

Link abbreviations used below:

- `TES/` = repo-relative paths in this worktree at `ecb9689`
- `LUA/` = `https://github.com/luanti-org/luanti/blob/9c424c44ff523ebce076367e52222ac4d71e94c1/`
- `SL/` = `https://github.com/PaperMC/Starlight/blob/cca03d62da48e876ac79196bad16864e8a96bbeb/`
- `SLL/` = `SL/src/main/java/ca/spottedleaf/starlight/common/light/`
- `SLM/` = `SL/src/main/java/ca/spottedleaf/starlight/mixin/common/`
- `GIST` = `https://gist.github.com/Spottedleaf/6cc1acdd03a9b7ac34699bf5e8f1b85c`
- `PAPER/` = `https://github.com/PaperMC/Paper/blob/df120fd8b5211f61146a59f69497a5a62064b917/paper-server/patches/sources/net/minecraft/` (the `-` lines of each patch are vanilla, Mojang-mapped)
- `FAWE/` = `https://github.com/IntellectualSites/FastAsyncWorldEdit/blob/c4c57f22d192c8b4d4d38be78dc4aef57cf04927/`
- `FAWEL/` = `FAWE/worldedit-core/src/main/java/com/fastasyncworldedit/core/extent/processor/lighting/`
- `WE/` = `https://github.com/EngineHub/WorldEdit/blob/130688b5d5705ff77d762b300ec8995414e1432c/`
- `MC23w03a` = `https://www.minecraft.net/en-us/article/minecraft-snapshot-23w03a`
- `MC23w17a` = `https://www.minecraft.net/en-us/article/minecraft-snapshot-23w17a`
- `MC25w02a` = `https://www.minecraft.net/en-us/article/minecraft-snapshot-25w02a`
- `VS/` = `https://github.com/anegostudios/vsapi/blob/63d33f70018c606fb0f70ead900fe90625fbd7b2/`
- `VSC/` = `https://github.com/anegostudios/vscreativemod/blob/1c649719734ac5c049ce947a945847b2a07e9f59/`
- `VEL/` = `https://gitlab.com/veloren/veloren/-/blob/7daa95fd53ae62aa185d31ad316d0cabfe2d70ac/`
- `GV/` = `https://github.com/Zylann/godot_voxel/blob/045ff9326a32fd10ac6fdac6357b5a098aaf2093/`
- `CRAFT/` = `https://github.com/fogleman/Craft/blob/d6888a6e1e54340358ce25b1220f410541360b6b/`
- `SOA1` = `https://web.archive.org/web/20210622035752/https://www.seedofandromeda.com/blogs/29-fast-flood-fill-lighting-in-a-blocky-voxel-game-pt-1`

Vanilla Minecraft has no official source. A claim about vanilla internals is cited from a Mojang changelog, from vanilla lines quoted in Paper's patches, or from a maintainer (Spottedleaf) describing it, and is marked **unconfirmed** otherwise. Third-party code that _calls_ vanilla (WorldEdit, Starlight mixins) confirms that an API exists and what it is named, not what it does inside.

---

## 0. Tesserae today: the Edit invariants a bulk path must keep

Every World change goes through `set_occupants`, and a Mod Command reaches it one Voxel at a time through `apply_intent` (`TES/crates/tesserae/src/server/edit.rs:1027-1048`). What one `set_occupants` call guarantees, each of which a bulk write must still guarantee:

| # | Invariant | Where |
| --- | --- | --- |
| I1 | **Never write light into a Chunk whose bulk solve is pending or running.** An Edit returns `Busy` while `is_unlit` (pending or relighting); the Mod drain skips that queue instead. | `edit.rs:741-745`, `mods/runtime.rs:234-238`, `streaming.rs:463-470` |
| I2 | **Incremental relight and bulk solve are not interchangeable.** `light_chunk` is a Gauss-Seidel max iteration that converges upwards only; two Chunks can hold each other's light with no source. Removal is only sound in `relight`, which tracks provenance (darken what the old level could have lit, regrow from the independent sources uncovered). | `TES/crates/tesserae-core/src/light.rs:857-875`, `streaming.rs:476-490` |
| I3 | **An Edit's own write never enters `unlit`, but poisons a running solve** of any Chunk it marks edited, so a stale bulk result cannot land over it. | `streaming.rs:476-496`, `streaming.rs:366-369` |
| I4 | **Mesh staleness covers four kinds of Chunk**: the edited one; every Chunk the relight wrote light into (`touched`); every Chunk across a boundary plane the relight wrote (`across`); and the face-neighbour when the Voxel sits on a boundary plane, even if no light moved. | `edit.rs:1062-1067`, `edit.rs:1111-1123`, `edit.rs:1133-1157`, `world_state.rs:349-355`, `world_state.rs:406-431` |
| I5 | **Canonical encoding.** Air drops shape/orientation; a cube or connected Shape drops orientation; Occupants encode to one (Voxel, Overlay) pair so equal-looking Voxels compare equal. Fluid and fire are vetted (`vet_occupants`). | `edit.rs:1078-1098`, ADR-0013 |
| I6 | **Shaped-Voxel cap per Chunk** (`MAX_SHAPED_PER_CHUNK`), checked against the Chunk's current count. | `edit.rs:1100`, `edit.rs:1160-1178` |
| I7 | **Relight only when the optical reading changes** (optical Voxel, wet, burning); a surface/reach change alone moves no light. | `edit.rs:1102-1123` |
| I8 | **Every change is journalled** (`note_changed`) to wake the fluid/fire simulation; past `JOURNAL_CAP` (4096) the journal coalesces to whole Chunks. Revisions bump for persistence. | `edit.rs:1124-1131`, `world_state.rs:63-65`, `world_state.rs:191-206` |
| I9 | **Level-1 cover invalidation** for the Chunk whose Voxels changed (`voxels_changed`), separate from light-only staleness. | `edit.rs:1133`, `streaming.rs:498-503` |
| I10 | **Snapshot isolation.** Every store write goes through `Arc::make_mut`, so a running Job never sees it and `Arc::ptr_eq` says whether a Chunk changed. | `world_state.rs:28-31`, `world_state.rs:259-270` |
| I11 | **Undo is per player Edit or per layout group**, keyed by Chunk `Lifetime`, refused if the cell no longer reads `after`; no-op entries are dropped; the journal holds at most 1024 Voxels. Mod Commands are **not** journalled for undo (only player Edits reach `EditHistory`/`Journal`). | `undo.rs:14-35`, `undo.rs:59-60`, `undo.rs:160-190`, `mods/runtime.rs:37-55` |
| I12 | **Logical steps.** A drained Mod Command is an event taking the index of the step under way; simulation landings go through the same `set_occupants`; bounds count operations, not time. | ADR-0014 lines 28-54 |

Why the cap is 2: at 256 the median worst drain was 10.585 ms against 0.296 ms at 2, and a full 32³ fill takes 16384 ticks, about 256 s at 64 Hz (`TES/DESIGN.md:442-447`). DESIGN names the follow-up explicitly: "no bulk write until relight can run once per Chunk without breaking Edit invariants" (`DESIGN.md:671-673`), and "a bulk relight is the named follow-up" for draining lava (`DESIGN.md:696-698`). The single-Voxel `relight` costs a darken queue plus a brighten queue per channel per Voxel (`light.rs:876-913`, `light.rs:951-995`, `light.rs:997-1027`), and every lattice access is a `HashMap` lookup by Chunk (`world_state.rs:396-411`), whereas `light_chunk` is "one contiguous sweep over all 32768 Voxels" (`light.rs:332-334`).

---

## 1. Minecraft (vanilla), Starlight, Paper, WorldEdit and FAWE

### Vanilla write path and flags

- **`Level.setBlock(pos, state, flags, updateLimit)` with bit flags.** Vanilla's constants, from the `-` lines of Paper's patch: `UPDATE_NEIGHBORS = 1`, `UPDATE_CLIENTS = 2`, `UPDATE_INVISIBLE = 4`, `UPDATE_IMMEDIATE = 8`, `UPDATE_KNOWN_SHAPE = 16`, `UPDATE_SUPPRESS_DROPS = 32`, `UPDATE_MOVE_BY_PISTON = 64`, `UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE = 128`, `UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS = 256`, `UPDATE_SKIP_ON_PLACE = 512`, composites `UPDATE_NONE = 260`, `UPDATE_ALL = 3`, `UPDATE_ALL_IMMEDIATE = 11`, `UPDATE_SKIP_ALL_SIDEEFFECTS = 816`, and `UPDATE_LIMIT = 512` (`PAPER/world/level/block/Block.java.patch` lines 7-20, 40). No flag names lighting: lighting is not a side effect a caller can opt out of through `setBlock`.
- **`LevelChunk.setBlockState(pos, state, flags)` honours `UPDATE_SKIP_ON_PLACE`** (`PAPER/world/level/chunk/LevelChunk.java.patch` line 137). That `LevelChunk.setBlockState` queues a light check through `LevelLightEngine.checkBlock` is **unconfirmed** from vanilla source; Starlight replaces exactly that entry point (below), and WorldEdit calls it itself after writing chunks directly (below).
- **Section storage.** `LevelChunkSection` holds a `nonEmptyBlockCount` and a `PalettedContainer` of states (`PAPER/world/level/chunk/LevelChunkSection.java.patch` line 7; Starlight reads `section.states` and `maybeHas` on the palette, `SLL/BlockStarLightEngine.java:212-237`). How `setBlockState` resizes the palette is **unconfirmed**.
- **`StructureTemplate.placeInWorld` writes block by block** through `level.setBlock(blockPos, state, updateMode)` (`PAPER/world/level/levelgen/structure/templatesystem/StructureTemplate.java.patch` lines 17, 44, 61); there is no bulk path there.
- **`/fill`, `/clone`, `/fillbiome` are capped by a game rule.** 23w03a: "Added `commandModificationBlockLimit`, controlling the maximum number of blocks changed in one execution of clone, fill and fillbiome" (`MC23w03a`). Its default (32768 by community sources) and a later rename are **unconfirmed** from a Mojang source.
- **`strict` skips updates, not light.** 25w02a: "fill, clone, setblock and place template accept a new option: strict. If specified, the command will place blocks as-is without triggering block updates and shape updates" (`MC25w02a`). Light is not listed.
- **Worldgen defers light to a LIGHT status.** Starlight drops light updates for a chunk not yet at `ChunkStatus.LIGHT` and runs them directly (no ticket) for a chunk below `FULL` (`SLM/lightengine/ThreadedLevelLightEngineMixin.java:53-69`); during feature generation "light data is not initialised" (`SLM/world/WorldGenRegionMixin.java:17-24`). That vanilla features write through `WorldGenRegion` into proto-chunks and light the whole chunk once at LIGHT is consistent with this, but vanilla's own implementation is **unconfirmed**.

### The 1.14 engine and the 1.20 rewrite

- **1.14–1.19**: Spottedleaf (Starlight's author) describes vanilla as recomputing a queued position's level _from all six neighbours_, with one update queue, a `Long2ByteMap` of pending values and a `LongLinkedOpenHashSet` queue so updates can be cancelled, plus per-update hashtable lookups for section initialisation and a changed-section set (`SL/TECHNICAL_DETAILS.md:197-274`, `:383-394`). He states his model is simplified and that he does not fully understand vanilla's decrease logic (`:61-67`). Treat details as his reading, not Mojang's.
- **1.20 rewrite (Mojang)**: 23w17a: "For some time we've been working on a new faster light engine ... community members that have developed mods and plugins which have highlighted these performance challenges and served as an inspiration for our new implementation", "Improved performance of the light engine", and fixed MC-172980 "Block light updates don't cross chunk borders properly when updated" and MC-245819 "Lighting can still occasionally lag behind world generation" (`MC23w17a`).
- **What 1.20 took from Starlight** (maintainer, `GIST`): two core ideas, "The light engine should propagate levels, rather than updates" and "The sky light engine should have dedicated logic to light chunks"; vanilla "now operates off of an increase queue and decrease queue, like Starlight", and is still about 2x slower but "fast enough". Starlight's own document says the 1.20 engine "copies basically everything from Starlight in this document and then some" (`SL/TECHNICAL_DETAILS.md:4-9`). The internal class layout (`LevelLightEngine`, `ThreadedLevelLightEngine.runLightUpdates`) after 1.20 is **unconfirmed** from source.

### Starlight: how it batches and why it is faster

- **Changes are queued per chunk, not propagated per block.** `checkBlock(pos)` is overwritten to enqueue `blockChange(pos)` (`SLM/lightengine/ThreadedLevelLightEngineMixin.java:117-127`), which adds the position to a `ChunkTasks.changedPositions` set keyed by chunk (`SLL/StarLightInterface.java:379-385`, `:583-587`). Duplicate positions collapse in the set.
- **One decrease + one increase per chunk task.** `propagateChanges` drains the queue chunk by chunk and calls `blocksChangedInChunk(..., positions, ...)` once per channel (`SLL/StarLightInterface.java:524-567`). The block engine calls `checkBlock` for _every_ position, each of which writes the new emission, enqueues an increase if it emits, and enqueues a decrease with the old level across all six directions; then it runs **one** `performLightDecrease` for the whole set (`SLL/BlockStarLightEngine.java:88-120`, `:198-204`), which ends in one `performLightIncrease` (`SLL/StarLightEngine.java:1292`, `:1571`).
- **Sky changes use a per-column heightmap of the batch.** For each changed column it takes the highest changed Y, re-propagates sky from above it, and queues decreases for every 15 below the first block sky cannot pass — once per column, not once per block (`SLL/SkyStarLightEngine.java:363-451`).
- **The propagator itself.** Push light to neighbours from the queued level, never recompute from six neighbours; decrease sets a neighbour to 0 if it is at or below what this source could have given, otherwise queues it for re-increase with a recheck flag (`SL/TECHNICAL_DETAILS.md:97-111`). Queue entries are packed `long`s carrying position, level, a 6-bit direction set and flags (`SLL/StarLightEngine.java:1037-1052`), in flat arrays (`:1055-1088`).
- **Measured.** One glowstone placed/removed in a lit section: Starlight 16523/20447 light gets and 4089 block gets; Paper (vanilla-like) 28623/152079 light gets and 49062/181452 block gets (`SL/TECHNICAL_DETAILS.md:323-343`). On 1.16.5, ~28x faster at glowstone removal, ~12x at placement, ~13x/~37x for block remove/place at y = 254 (`:436-444`).
- **Light sections tied to chunks.** "Starlight assumes that no updates are possible unless the chunk exists"; light state lives on `ChunkAccess`, so the chunk system need not track light engine state (`SL/README.md:41-59`).
- **Whole-region relight exists as a separate tool.** `relightChunks(set)` lights each requested chunk and every neighbour within distance 2 from empty light (`getFilledEmptyLight`), and writes nibbles back only for the requested chunks; the set should be "ordered according to a BFS from the center" (`SLL/StarLightEngine.java:915-1035`, `:994`, `:1021`). That neighbours are solved only as a rim and not written back is my reading of the code.
- **Status.** Starlight is unmaintained as a mod past 1.20; it lives on in Paper and Folia via Moonrise (`GIST`, `SL/README.md:5-7`). Paper-specific block-change batching beyond this was not investigated (**unconfirmed / not covered**).

### WorldEdit (EngineHub): side effects, delayed or off

- `SideEffect` has `LIGHTING`, `NEIGHBORS`, `UPDATE`, `VALIDATION`, `ENTITY_AI`, `EVENTS` and internal `POI_UPDATE`, `NETWORK`; each is `OFF`, `ON` or `DELAYED` (`WE/worldedit-core/src/main/java/com/sk89q/worldedit/util/SideEffect.java:24-38`, `:74-77`).
- **Direct chunk writes, then a light check per position.** The Mojang-mapped adapter writes `chunk.setBlockState(pos, state, flags)` with `UPDATE_SKIP_ON_PLACE | UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE` when updates are off, and implements lighting as `level.getChunkSource().getLightEngine().checkBlock(pos)` (`WE/worldedit-core-mc/src/main/java/com/sk89q/worldedit/coremc/internal/CoreMcWorldNativeAccess.java:80-104`).
- **Delayed side effects are replayed position by position, time-sliced.** With `postEditSimulation`, `setBlock` records each position's old state and writes with no side effects; the commit operation then calls `applySideEffects` per position while `run.shouldContinue()` and resumes later (`WE/worldedit-core/src/main/java/com/sk89q/worldedit/extent/world/SideEffectExtent.java:90-128`). `ChunkBatchingExtent` buffers writes in a `BlockMap` to reorder them by chunk (`WE/worldedit-core/src/main/java/com/sk89q/worldedit/extent/reorder/ChunkBatchingExtent.java`).

### FAWE: fast placement, then relight by chunk

- **Fast placement** "Place chunks instead of individual blocks" (`FAWE/worldedit-core/src/main/java/com/fastasyncworldedit/core/configuration/Settings.java:429-434`).
- **Relight modes**: `NONE`, `OPTIMAL` ("relight changed light sources and changed blocks"), `ALL` ("relight every single block") (`FAWEL/RelightMode.java`); config adds `DELAY_PACKET_SENDING` ("If packet sending should be delayed until relight is finished"), `ASYNC`, and `REMOVE_FIRST` ("If existing lighting should be removed before relighting") (`Settings.java:838-855`).
- **Per-chunk, not per-block, after the write.** `RelightProcessor` adds a chunk to the relighter once per chunk written, with a per-section skip mask for sections the edit did not touch (`FAWEL/RelightProcessor.java`). Block light: changed positions are kept as per-chunk bitsets and relit with SoA-style propagation and removal queues (`FAWEL/NMSRelighter.java:62-64`, `:134-158`, `:209-260`).
- **Remove-first clears only interior chunks.** With `REMOVE_FIRST`, sky light is removed only from chunks whose four horizontal neighbours are also in the set, so edge chunks keep their light and act as the rim it refills from (`FAWEL/NMSRelighter.java:993-1023`).
- **On Paper it hands whole chunks to Starlight.** Each edited chunk adds its 3x3 neighbourhood ("light can go into neighboring chunks, make sure they are relighted too"), grouped into batches of 32x32 chunks, then `starlight$serverRelightChunks` (`FAWE/worldedit-bukkit/src/main/java/com/fastasyncworldedit/bukkit/adapter/StarlightRelighter.java:36`, `:118-133`; `.../adapter-26.3/.../PaperweightStarlightRelighter.java:59`).

---

## 2. Luanti (Minetest)

### API (`LUA/doc/lua_api.md`)

- **`set_node`**: sets one node, deletes metadata (`:7004-7010`). **`bulk_set_node`**: "Faster than set_node due to single call, but still considerably slower than Lua Voxel Manipulators (LVM) for large amounts of nodes. Unlike LVMs, this will call node callbacks ... For setting a cube, this is 1.3x faster than set_node whereas LVM is 20 times faster" (`:7012-7021`). **`swap_node`** keeps metadata and runs no con-/destructor callbacks (`:7022-7027`).
- **VoxelManip**: `read_from_map(p1, p2)` loads a MapBlock-aligned region; `set_data` stages it; `write_to_map([light])` writes it back and "if `light` is true, then lighting is automatically recalculated ... If `light` is false ... you should correct all modified blocks with `core.fix_light()` as soon as possible" (`:5701-5717`). `calc_lighting` and `set_lighting` are only for the mapgen VM (`:5727-5762`); `update_liquids` must be called for liquids to start flowing (`:5645-5650`, `:5763`); `update_map` "Does nothing, kept for compatibility" (`:5726`).
- **Schematics**: `place_schematic` and `place_schematic_on_vmanip` (`:8138-8170`).

### C++ (`LUA/src/`)

- **Why `set_node` is slow.** `ServerEnvironment::setNode` runs the Lua `on_destruct`, `addNodeWithEvent`, `after_destruct` and `on_construct` callbacks around every node (`serverenvironment.cpp:597-628`). `Map::addNodeAndUpdate` records a rollback action and, unless the lighting flags of old and new are equal, zeroes the node's light and calls `voxalgo::update_lighting_nodes` with **one** old node (`map.cpp:162-215`). `addNodeWithEvent` dispatches one `MapEditEvent` per node with its modified blocks, which drives re-send to clients (`map.cpp:223-240`). `bulk_set_node` is just a loop over `env->setNode` (`script/lua_api/l_env.cpp:183-211`).
- **Skip relight when nothing optical changed.** `if (f == oldf)` the old light is copied over and no light update runs (`map.cpp:182-190`).
- **`update_lighting_nodes` is already batch-capable.** It takes a vector of `(pos, oldnode)`, computes a `min_safe_light` from the brightest old node (bumped by one when more than one node changed), seeds one unlight queue and one relight queue for all of them (including sky columns above/below), then runs one `unspread_light` and one `spread_light` per bank (`voxelalgorithms.cpp:454-632`). Luanti's liquid transform uses it that way: every liquid change of a step is collected and relit in a single call (`servermap.cpp:1195-1257`).
- **Queues bucketed by level.** `LightQueue` keeps one vector per light level and returns the brightest first (`voxelalgorithms.cpp:63-75`).
- **`blit_back_with_light`: the region relight.** Used by `write_to_map(true)` (`script/lua_api/l_vmanip.cpp:146-174`) and by `Schematic::placeOnMap` (`mapgen/mg_schematic.cpp:222-265`). Steps (`voxelalgorithms.cpp:1030-1138`):
  1. Reset the whole VoxelManip to "sunlight from above or dark", and propagate sunlight _and shadow_ below the manipulated region block by block.
  2. For every node on the **border shell** of every MapBlock in the region, compare old and new light; where the new is dimmer, push the old level onto the unlight queue.
  3. Blit the data back into the map.
  4. `finish_bulk_light_update`: unspread from the border seeds (which reaches outside the region), then collect **every** node in the region with light > 1 as a relight seed, and spread once (`voxelalgorithms.cpp:964-1028`).
- **Mapgen lighting** is a straight sunlight column pass (`propagateSunlight`, stopping at the first node that does not pass sunlight) followed by one flood (`spreadLight`) over the generated area (`mapgen/mapgen.cpp:466-509`).

---

## 3. Vintage Story (API only; engine closed)

- **Bulk accessor**: `GetBlockAccessorBulkUpdate(synchronize, relight)` "Does not to relight/sync on a SetBlock until Commit() is called. On commit all touched blocks are relit/synced at once. This method should be used when setting many blocks (e.g. tree generation, explosion, etc.)" (`VS/Common/API/IWorldAccessor.cs:722-729`). `Commit()`: "Sets all blocks, relight all affected one chunks in one go and send blockupdates to clients in a packed format" (`VS/Common/API/IBlockAccessor.cs:560-567`). Staged writes are a `Dictionary<BlockPos, BlockUpdate>`, so a second write to one position replaces the first, and reads can come from the stage (`VS/Common/API/IBulkBlockAccessor.cs:13-41`).
- **A no-relight variant** (`GetBlockAccessorBulkMinimalUpdate`, used for snow accumulation) (`IWorldAccessor.cs:731-737`).
- **Undo built into the accessor**: `GetBlockAccessorRevertable` is "Same as GetBlockAccessorBulkUpdate, additionally, each Commit() stores the previous state and you can perform undo/redo operations on these" (`IWorldAccessor.cs:745-751`); it exposes `Relight`, `CurrentHistoryState`, `ChangeHistoryState(n)`, `QuantityHistoryStates` ("More states means more memory usage") (`VS/Common/API/IBlockAccessorRevertable.cs`).
- **Solid and fluid layers recorded separately per Voxel.** `BlockUpdate` holds `OldBlockId`, `OldFluidBlockId`, `NewSolidBlockId`, `NewFluidBlockId` (negative means "this layer unchanged") and block-entity bytes (`VS/Common/API/IBlockAccessor.cs:10-35`); `PostCommitCleanup` exists "to fix certain things like flowing water from the edge of a pasted schematic/selection when undone" (`IBulkBlockAccessor.cs:49-53`).
- **WorldEdit (open, in `vscreativemod`)** uses the revertable accessor with `Relight = DoRelight`, toggled by `/we relight` (`VSC/Workspace.cs:96`, `:356-358`, `VSC/WorldEditCommands.cs:1182-1192`), commits once per operation and calls `PostCommitCleanup` after a delete (`VSC/Workspace.cs:607-612`). Its overload guard rejects placing more than 100 light-emitting blocks at once, or more than 200³ blocks (`VSC/Workspace.cs:616-645`).
- How `Commit` relights inside the engine is **unconfirmed** (closed source).

---

## 4. Veloren

- **No stored light; light is computed in the mesher.** `calc_light` floods a light map over the chunk's bounds grown by `SUNLIGHT` (24) on each horizontal side, starting sun rays from the top and BFS-propagating, then keeps only the in-chunk part; `generate_mesh` calls it for sun and for glow (`VEL/voxygen/src/mesh/terrain.rs:32-226`, `:228-262`).
- **Edits are batched per tick.** `BlockChange` is a `HashMap<pos, Block>` (`set` overwrites, `try_set` first-wins) (`VEL/common/state/src/state.rs:61-82`); `apply_terrain_changes` takes the whole map once a tick, merges scheduled changes, applies them, and publishes `TerrainChanges { modified_chunks, modified_blocks, ... }` (`state.rs:109-123`, `:676-843`).
- **Remesh, not relight.** The client walks `modified_blocks` and inserts the chunk and every neighbour within `MAX_LIGHT_DIST` into a `mesh_todo` map (deduped by chunk), skipping neighbours when `skip_remesh` says neither light nor colour changed; a TODO notes this remeshes more than needed (`VEL/voxygen/src/scene/terrain/mod.rs:928-1010`).

## 5. Godot Voxel (Zylann)

- **Area ops are one edit each.** "The following methods represent one edit each ... using `do_box` will be more efficient than calling `do_point` many times" (`GV/edition/voxel_tool.h:71-77`). `paste` writes a `VoxelBuffer` into storage and calls `_post_edit` once for its box (`GV/edition/voxel_tool_terrain.cpp:64-71`, `:245-248`); `post_edit_area` marks the area modified, optionally sends the whole box to multiplayer peers, and schedules mesh updates for the box padded by one voxel (`GV/terrain/fixed_lod/voxel_terrain.cpp:830`, `:841-866`).
- **No flood-fill light.** `VoxelBuffer`'s channels are type, SDF, colour, indices, weights and three data channels, none for light (`GV/storage/voxel_buffer.h:40-51`); the blocky mesher exposes only baked ambient occlusion (`occlusion_enabled`, `occlusion_darkness`) (`GV/doc/source/api/VoxelMesherBlocky.md:17-18`). That it has no voxel light at all is from absence in the API and docs (**unconfirmed** as a positive statement).

## 6. Craft (other)

- Craft computes block light **per chunk at mesh time** by flooding from every light in the 3x3 chunk neighbourhood, and an edit to a chunk with lights dirties its eight neighbours (`CRAFT/src/main.c:865-877`, `:958-1022`). Like Veloren it pays a full re-flood per remesh rather than storing and patching light.
- Cubiquity, Terasology, Vengi, Teardown and Hytale were not researched (**not covered**).

---

## 7. Light propagation algorithms

- **Add queue (BFS).** Set the source, enqueue it; pop a node, and for each neighbour whose light is at least two below, set it to level − 1 and enqueue it (`SOA1`, "The Algorithm", steps 1-5; author "Benjamin - Lead Developer", Seed of Andromeda). Each lit voxel is visited once: "We only want to visit each voxel that the light touches ONE time!" (`SOA1`).
- **Removal queue.** Enqueue the removed source with its old value and zero it; pop a node and for each neighbour: if its light is non-zero and _less_ than the node's, zero it and enqueue it on the removal queue; "Else if it is >= current node, then add it to the light propagation queue", which is run after the removal queue empties, "so that if you have two lights next to each other, removing one light will allow the other light to fill in the empty spaces left behind" (`SOA1`, "Light Removal"). Starlight cites this post as where it learned decreases (`SL/TECHNICAL_DETAILS.md:113-117`). Tesserae's `darken`/`brighten` is this algorithm with a sharper test (`light.rs:951-995`).
- **Batching is free in the algorithm.** Both queues are multi-source: Starlight seeds all changed positions into one decrease queue before one pass (`SLL/BlockStarLightEngine.java:198-204`); Luanti does the same in `update_lighting_nodes` and in the liquid step (`voxelalgorithms.cpp:454-632`, `servermap.cpp:1257`). The removal of overlapping regions is then paid once, not per voxel.
- **Level-ordered queues.** Draining brightest first means each cell spreads once; Luanti's `LightQueue` is bucketed by level (`voxelalgorithms.cpp:63-75`) and Tesserae's bulk `flood` already drains per level for the same reason (`light.rs:704-709`), but its incremental `brighten` is one FIFO (`light.rs:997-1027`), so with many seeds a cell can be raised several times.
- **Sunlight columns.** Full sky passes straight down undimmed (Tesserae `transmitted`, `light.rs:283-301`; Luanti `propagateSunlight`, `mapgen.cpp:476-509`; Starlight's per-column heightmap, `SkyStarLightEngine.java:363-451`). A roof placed over open sky must darken the whole column below it, which can leave any bounded region; Luanti propagates "sunlight and shadow below the voxel manipulator" for this (`voxelalgorithms.cpp:1046-1070`).
- **Region relight (clear + rim + re-flood).** Luanti `blit_back_with_light` and FAWE `REMOVE_FIRST` both reset the interior, keep the rim, unlight outward where the border got darker, then re-flood from every source inside plus the rim (§1, §2). It is correct for removals only because the unlight pass at the border removes light that the old interior had pushed outside.
- **Cost models.** Per-voxel incremental: work ∝ Σ over edits of the voxels each one's light reached (~(2·15+1)³ worst case per emitter edit; a sky column per roof edit), with random-access reads. Batched incremental: work ∝ the _union_ of those regions, each cell darkened/raised a bounded number of times. Region relight: work ∝ region volume (every voxel scanned, `finish_bulk_light_update` walks every node, `voxelalgorithms.cpp:984-1006`) plus the rim unlight, with contiguous access. Starlight's measurements show the per-update constant (hash lookups, neighbour reads) dominates, not the asymptotics (`SL/TECHNICAL_DETAILS.md:276-401`).

---

## 8. Comparison against Tesserae's constraints

| Approach | Write unit | Relight unit | Removal sound? | Undo | Fits ADR-0002 (Server only) | Fits core without Bevy | Fluids beside Voxel (ADR-0013) | Bounded step (ADR-0014) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| MC vanilla `setBlock` per block | block | queued per block, run later (**unconfirmed** detail) | yes | none in engine | yes | n/a | fluid is part of state | no bound on `/fill` beyond the game rule |
| Starlight queue | block, deduped per chunk | one decrease+increase per chunk per drain | yes (SoA removal) | none | yes | algorithm is engine-agnostic | n/a | drained in one go |
| Starlight / FAWE region relight | chunk | 3x3 or BFS-ordered chunk set, from empty light | yes for requested chunks (rim solved from scratch) | FAWE history | yes | yes | n/a | FAWE batches 32x32 chunks |
| WorldEdit `DELAYED` | block | per position, time-sliced | yes (vanilla) | WE history | yes | n/a | n/a | yes, resumable |
| Luanti `set_node` | node | one-node `update_lighting_nodes` | yes | rollback log | yes | n/a | liquids separate | no |
| Luanti VoxelManip | MapBlock region | region reset + border unlight + one spread | yes | none | yes | yes | `update_liquids` separate step | no, one call |
| VS bulk/revertable accessor | staged dict | per chunk on `Commit` (**unconfirmed** how) | **unconfirmed** | built in, per commit, solid+fluid | yes | n/a | solid and fluid layers distinct | caller decides |
| Veloren | per-tick map | none stored; full re-flood per remesh | trivially (recomputed) | none | yes | n/a | n/a | per tick |
| Godot Voxel | box/buffer | no light; remesh padded box | n/a | user-side | n/a | n/a | n/a | n/a |

Against Tesserae specifically: the one thing none of the "light at mesh time" engines (Veloren, Craft, Godot) can teach is removal, because they never store light; Tesserae stores light per Voxel and must keep I2. The engines that store light (Minecraft/Starlight, Luanti) both converge on **multi-source SoA queues for scattered changes** and **region reset + rim unlight + one flood for dense regions**.

---

## 9. What this suggests for Tesserae

### Option A: batched incremental relight (`relight_many`) — recommended first

Write a batch of Voxels for one tick, then relight them in **one** darken pass and **one** brighten pass per channel over the union, as Starlight (`BlockStarLightEngine.java:198-204`) and Luanti's `update_lighting_nodes` (`voxelalgorithms.cpp:454-632`) do.

- Shape in core: `relight_many(volume, tables, edits: &[([i32; 3], Light)])` beside `relight`, in `tesserae-core` with no new dependency (constraint 3). Set every edited Voxel's `now` first, then seed all `was > now` into one darken queue, then collect all regrow seeds and brighten once. `relight` becomes the one-element case, so the "two solvers agree" tests (`light.rs:1538` and siblings) extend to "batched = sequential = fresh solve".
- Keeps I2 exactly: still provenance-tracking, so removal stays sound across Chunks; no `light_chunk` mixing.
- Make `brighten` drain brightest first (per-level buckets), as the bulk `flood` already does (`light.rs:704-709`) and Luanti's `LightQueue` does (`voxelalgorithms.cpp:63-75`); with many seeds FIFO re-raises cells.
- Mesh marking (I4) falls out of `LightLattice.touched`/`across` once per batch; boundary-neighbour marking per written Voxel stays; `MeshState::dirty` is a set, so each Chunk is re-meshed once.
- Trade-off: work is ∝ the union of light regions, with `HashMap` lookups per access (`world_state.rs:396-411`). For a dense 32³ fill of emitters it approaches a Chunk's volume times a hashed constant — fine for scattered builds, worse than a sweep for full Chunks. Cheap to add: one per-batch Chunk cache in `LightLattice` removes most hashing (Starlight caches a chunk neighbourhood for the same reason, `StarLightEngine.java:172-255`).

### Option B: region relight for dense batches (clear + rim unlight + re-flood)

When a batch covers most of a Chunk, reset the touched Chunks' light and re-flood, following Luanti's `blit_back_with_light` (`voxelalgorithms.cpp:1030-1138`) and FAWE's interior-only `REMOVE_FIRST` (`NMSRelighter.java:993-1023`).

- Must **not** be "mark the Chunks unlit and let `light_chunk` re-solve": that is the Gauss-Seidel trap in I2 (`light.rs:868-875`), since light the old interior pushed into untouched neighbours would stay. The sound version: (1) record the boundary plane of each touched Chunk; (2) clear the interior; (3) darken outward from the boundary where old > new, exactly as Luanti seeds its unlight queue from block borders, including the sky column below; (4) seed from emitters, open sky and the neighbours' facing planes; (5) one flood. Steps 3-5 run on `LightLattice`, so this is Option A's machinery with a different seed set.
- Can reuse the bulk sweep (`light_cells`, `light.rs:352`) for step 4-5 inside a single Chunk whose neighbours are all untouched; across several touched Chunks the lattice flood is simpler than iterating Gauss-Seidel to a fixed point.
- Trade-off: cost ∝ region volume regardless of how little light moved; best for fills and `Fill` Commands, worse for sparse `Place` lists. Choose A vs B per Chunk by how many of its Voxels the batch writes.

### Option C: bulk write that keeps the other invariants

Independent of the relight choice, the write itself (I5-I10) can be batched:

- **Group by Chunk, vet per Voxel, cap cumulatively.** `vet_occupants` and canonical encoding stay per Voxel; `vet_capacity` must count shaped units added earlier in the same batch (I6), or a batch can overshoot the cap that one-at-a-time would have refused.
- **Dedupe no-op writes.** Drop a write whose encoded (Voxel, Overlay) equals the stored one before it costs a journal entry, a mesh mark or a revision bump. Today `set_occupants` skips `set_voxel` on equality but still marks the Chunk edited and journals (`edit.rs:1103-1105`, `:1127-1134`). Luanti skips light when lighting flags are equal (`map.cpp:182-190`), Veloren skips remesh when neither light nor colour changed (`voxygen/src/scene/terrain/mod.rs:936-955`), and VS and Veloren dedupe by position in a map (`IBulkBlockAccessor.cs:20`, `state.rs:61-82`).
- **One `note_changed` per Voxel is fine**: past 4096 the journal already coalesces to Chunks (I8), which is the right behaviour for a fill.
- **I1 widens.** A batch must check `is_unlit` for every Chunk it writes _and_ every Chunk its light can reach (one Chunk ring, since `MAX_LIGHT` 15 < 32, `light.rs:22-25`), or defer just the Voxels in those Chunks; today the per-Voxel check only covers the edited Chunk and relies on poisoning (I3) for neighbours, which still applies.
- **Undo.** Mod Commands are not undoable today (I11); if player layouts move onto the bulk path, each layout stays one `Group` with per-Voxel before/after Occupants (solid and fluid together, as VS's `BlockUpdate` records both layers, `IBlockAccessor.cs:10-35`), and undo of a group can itself use the bulk path.

### Option D: drain order and budget

- **Budget in Voxels per Chunk-batch, not per Voxel.** Replace `MOD_VOXELS_PER_TICK = 2` by a budget that admits one Chunk's worth of a queue's front Command per tick (or N Voxels grouped by Chunk), measured the same way the cap of 2 was (`DESIGN.md:442-447`). A bound that counts operations keeps ADR-0014's rule; each drained batch is one event at the step under way (I12).
- **Nearest-first and top-down.** Starlight recommends BFS order from the centre for region relights (`StarLightEngine.java:917-918`); Tesserae already solves higher layers first because "sky light flows downwards" (`streaming.rs:349-352`). Draining a Fill by Chunk, nearest the Eye first and highest first within it, avoids relighting a column twice. FIFO per Mod with round-robin across Mods (`mods/runtime.rs:213-251`) can stay as the fairness layer above it.
- **Emitter-heavy batches.** VS's WorldEdit refuses >100 emitters at once (`VSC/Workspace.cs:621`); Tesserae's simulation bounds emitter changes to 4 a step (`GLOSSARY.md:314-316`). A Mod fill of lava needs its own emitter bound, or Option B for that Chunk.

### Option E: deferred relight queue (later, if A/B are not enough)

Queue changed positions per Chunk and relight them later in the tick or on a worker, as Starlight's `LightQueue` (`StarLightInterface.java:570-650`), FAWE's `DELAY_PACKET_SENDING`/`ASYNC` (`Settings.java:838-851`), VS's `Commit` and WorldEdit's `DELAYED` (`SideEffectExtent.java:109-128`) do.

- Needs a new "light pending (incremental)" state distinct from `unlit`, because `unlit` routes to `light_chunk` (I2); meshes of those Chunks wait, and Edits into them return `Busy` as with I1.
- Off-thread relight (ADR-0007 Jobs) would have to snapshot the lattice and be poisoned by later Edits like a bulk solve (I3); the gain is the tick's CPU, the cost is another state machine. Not worth it until A/B are measured.

### Recommendation

1. **Option A + C + D** first: a `tesserae-core` `relight_many` (multi-source darken, level-bucketed brighten), a bulk `set_occupants_many` on the Server that groups by Chunk, dedupes no-ops, vets cumulatively and calls `relight_many` once, and a drain budget counted per Chunk-batch nearest-first. It keeps every invariant in §0 without a new light state, and its correctness is testable against the existing solver-agreement tests.
2. **Option B** as the dense-Chunk path once A is measured on a full 32³ fill and on lava draining at 0.25 m (the costliest step today, `DESIGN.md:696-698`).
3. **Option E** only if A/B still cannot fit the tick.

## Unconfirmed or not covered

- Vanilla: that `LevelChunk.setBlockState` calls `LevelLightEngine.checkBlock`; the queue and `runLightUpdates` internals of `ThreadedLevelLightEngine`; the post-1.20 class layout; palette resizing in `LevelChunkSection.setBlockState`; that features write through `WorldGenRegion` into proto-chunks with light deferred to the LIGHT status (inferred from Starlight's mixins only); the `/fill` limit's default (32768) and its later renaming.
- Vanilla 1.14–1.19 propagation details are Spottedleaf's simplified model, by his own account.
- Paper's own block-change batching beyond Starlight/Moonrise: not investigated.
- Starlight `relightChunks` writing back only the requested chunks' light: my reading of the code.
- Vintage Story: how `Commit()` relights and whether removal is exact (engine closed).
- Godot Voxel having no voxel light at all: from absence in API and docs.
- Seed of Andromeda Part 2 (sunlight, coloured light): the archive was offline when fetched; not read.
- Cubiquity, Terasology, Vengi, Teardown, Hytale: not researched.
