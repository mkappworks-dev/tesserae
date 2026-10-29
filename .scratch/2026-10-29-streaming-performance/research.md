# Voxel streaming, culling and LOD: how six engines do it (research for Tesserae)

Date: 2026-10-29. Read-only research; no repo edited.

## Sources pinned

| Project | Source | Commit / version |
| --- | --- | --- |
| Sodium | github.com/CaffeineMC/sodium | `148151cf25c56f8b670986b348dcc589278c867e` (2026-10-16) |
| Luanti | github.com/luanti-org/luanti | `054e2b17ee769305dc99ad3b1cdae049cd913f26` (2026-10-29) |
| Voxy | github.com/MCRcortex/voxy | `5bf33350a1cc7bb7e2b5d947b236c56d878b8a7d` (2026-10-19) |
| Distant Horizons core | gitlab.com/distant-horizons-team/distant-horizons-core | `ba795edb914d7d44a87fbb8eb8957efee5125424` (2026-10-27) |
| Distant Horizons (MC glue) | gitlab.com/distant-horizons-team/distant-horizons | `bbf5730983ac5845abc2749df829f6a8213716a0` |
| Vintage Story API (engine itself is closed source) | github.com/anegostudios/vsapi | `63d33f70018c606fb0f70ead900fe90625fbd7b2` |
| Bevy | crates.io `bevy_* 0.20.0` (Tesserae `Cargo.lock`) | tag `v0.20.0` |
| wgpu | crates.io `wgpu / wgpu-hal / wgpu-types 30.0.1` (used by Bevy 0.20 in Tesserae's lock) | `v30.0.1` |

Link abbreviations used below:

- `SOD/` =
  `https://github.com/CaffeineMC/sodium/blob/148151cf25c56f8b670986b348dcc589278c867e/common/src/main/java/net/caffeinemc/mods/sodium/client/`
- `LUA/` =
  `https://github.com/luanti-org/luanti/blob/054e2b17ee769305dc99ad3b1cdae049cd913f26/`
- `VOXY/` =
  `https://github.com/MCRcortex/voxy/blob/5bf33350a1cc7bb7e2b5d947b236c56d878b8a7d/src/main/`
- `DH/` =
  `https://gitlab.com/distant-horizons-team/distant-horizons-core/-/blob/ba795edb914d7d44a87fbb8eb8957efee5125424/`
- `DHMC/` =
  `https://gitlab.com/distant-horizons-team/distant-horizons/-/blob/bbf5730983ac5845abc2749df829f6a8213716a0/`
- `VS/` =
  `https://github.com/anegostudios/vsapi/blob/63d33f70018c606fb0f70ead900fe90625fbd7b2/`
- `BEVY/` = `https://github.com/bevyengine/bevy/blob/v0.20.0/crates/`
- `WGPU/` = `https://github.com/gfx-rs/wgpu/blob/v30.0.1/` (I read the crates.io
  30.0.1 sources locally; the GitHub tag path was not checked)

Vanilla Minecraft has no official source. Claims about its internals are marked
**unconfirmed** unless an author-written source covers them.

---

## 1. Section visibility culling

### Minecraft (vanilla)

- **Face-pair connectivity, 15 bits per 16³ chunk.** A flood fill from each
  non-opaque block records which faces it reaches. Each chunk stores "just 15
  bits… one for each possible pair of faces", and the graph is rebuilt in the
  background whenever an opaque block changes, at about 0.1–0.2 ms per chunk.
  Author post (Tommaso Checchi, Mojang):
  https://tomcc.github.io/2014/08/31/visibility-1.html
- **BFS from the camera's chunk.** The walk only steps through faces with `N·V <
  0` ("We never want to go back"). A neighbour C is visited only if the current
  chunk B's graph connects the face it was entered through (from A) to the face
  leading to C. The frustum test runs last because it costs the most. A step
  budget adds +1 for going down below sea level and +3 for a fully dark chunk.
  https://tomcc.github.io/2014/08/31/visibility-2.html
- **Measured numbers (same post).** Raycasting was rejected: about 17k chunks ×
  7 rays ≈ 119k rays per frame. The step budget adds 5–15% to the cull ratio.
  Before the BFS, frustum culling, sorting and rebuild scheduling together took
  about 14% of frame time.
- **Mojang's class names.** Sodium imports
  `net.minecraft.client.renderer.chunk.VisibilitySet` and encodes it into a
  64-bit mask, using `bit(from,to) = from*8+to`.
  `SOD/render/chunk/occlusion/VisibilityEncoding.java` L8-24. Vanilla's own
  early-out thresholds are **unconfirmed** (no official source).

### Sodium

- **Directional vis-graph instead of vanilla's.** `DirectionalVisGraph` resolves
  each 16³ section into up to 4 "base perspective" VisibilitySets, using flood
  fills that only move along 3 allowed axis directions. The BFS then picks the
  set(s) that match the camera octant (`joinVisibilityData`). Early-outs: all
  4096 blocks opaque means nothing is visible; fewer than 256 opaque blocks
  means all faces connect. `SOD/render/chunk/occlusion/DirectionalVisGraph.java`
  L17-31, L49-66; `OcclusionCuller.java` L340-370.
- **Angle masks.** A straight-through path (e.g. down→up) is removed when the
  section is viewed at an angle where its far side can't be seen: `dx > dy +
  margin`, with `margin = 32*width - 16`. `OcclusionCuller.java` L55-108.
- **Three BFS results per run.** Each run produces WIDE (valid while the camera
  stays within ±1 section), REGULAR (anywhere in the current section) and LOCAL
  (exact camera, frustum- and fog-tested, plus a ray test). The renderer uses
  the narrowest tree still valid for the current camera. LOCAL is thrown away
  when the camera moves. `occlusion/CullType.java` L3-20;
  `render/chunk/RenderSectionManager.java` L181-202, L274-288.
- **Async culling.** The BFS runs on a "Sodium Async Cull Thread" with
  cancellation. If the camera changed after submission, the result is not used.
  `RenderSectionManager.java` L215-262.
- **Ray occlusion inside the BFS (LOCAL tree).** A ray steps from the section
  centre toward the camera, at most 12 steps, only for sections more than 48
  blocks away. The section is culled if the ray leaves the set of sections the
  BFS has already traversed ("portals").
  `occlusion/RayOcclusionSectionTree.java` L13-16, L52-80.
- **Fog culling.** Only the LOCAL tree is fog-culled: its search distance is the
  effective fog end when `useFogOcclusion` is on. `RenderSectionManager.java`
  L356-374.
- **Per-face culling at draw time.** Each section's geometry is split into 7
  facing slices. Slices facing away from the camera are skipped when the draw
  commands are built. `render/chunk/DefaultChunkRenderer.java` L223-235, L342+.

### Luanti

- **Server-side occlusion before sending.** Rays go from the eye to the block's
  centre and its 8 corners, overshooting by 1 node. Each ray takes 1.2 m steps
  growing ×1.05, and a ray counts as blocked only after 2 light-blocking nodes.
  In "dense" mode (far blocks, `d >= block_cull_optimize_distance`), unloaded
  blocks near the target count as opaque. `LUA/src/map.cpp` L670-791.
- **Settings.** `server_side_occlusion_culling` (default true) "can reduce the
  number of blocks sent to the client by 50-80%". `block_cull_optimize_distance`
  (default 25 mapblocks) switches to the cheaper checks.
  `LUA/builtin/settingtypes.txt` L2377-2385.
- **Client BFS (default `occlusion_culler = bfs`).** The BFS walks only "far"
  sides. It uses 6 `solid_sides` bits per block (a face is solid only if every
  node on it is fully solid), plus dominant-axis rules and a raytraced check per
  block (`enable_raytraced_culling`, default true). `loops` is the old O(n³)
  culler. `LUA/src/client/clientmap.cpp` L399-700; `settingtypes.txt` L667-677;
  `src/client/mapblock_mesh.cpp` L763-786.
- **Coarse face bits, not face pairs.** Luanti stores 6 per-face "fully opaque"
  bits, not 15 face-pair bits. Client occlusion culling is turned off when
  `mesh_grid.cell_size >= 4` ("highly inefficient at these sizes"). The draw
  list is not rebuilt every frame, so the frustum test uses a padded sphere: +30
  nodes × BS. `clientmap.cpp` L419-425, L497-503.

### Vintage Story (closed-source engine)

- **Raycasting rather than BFS (2018).** "2 rays for each chunk… on every chunk
  at the edge of the view distance", about 2,300 rays for a 320 m view distance,
  versus the 119k cited for Minecraft. The author found BFS "very
  underwhelming". Off by default at the time. v1.7.2 dev blog:
  https://www.vintagestory.at/blog.html/news/the-search-for-excellence-v172-r162/
- **Chunks are 32³.** `GlobalConstants.ChunkSize = 32` ("Now a hard-coded
  constant"). `VS/Config/GlobalConstants.cs` L39. The current culling
  implementation is **unconfirmed**: the engine is closed source.

## 2. Work prioritisation

### Luanti (server → client block sending)

- **FOV padding, the exact code.** `getFovCosSq` uses `cos(camera_fov * 0.55f)`,
  a "HOTFIX: use slightly increased angle (+10%)", since half the FOV would be
  0.5. `getFovAdj` moves the cone apex back by `BLOCK_MAX_RADIUS / sin(fov/2)`,
  so a block counts as in sight if any part of it is. `LUA/src/util/numeric.h`
  L338-354; test in `src/util/numeric.cpp` L95-140.
- **Movement narrows the send cone and adds a forward cone.** `camera_fov /= 1 +
  dot(camera_dir, velocity)/300`, capped at a 50% reduction. When speed exceeds
  1 node/s, blocks inside a 0.1 rad (about 5°) cone along the velocity are also
  accepted. `LUA/src/server/clientiface.cpp` L229-237, L284-298.
- **Ordering.** Blocks are visited shell by shell, `d = 0..max`, using
  `FacePositionCache`. The scan resumes from the nearest unsent shell and resets
  when the camera turns by more than 10% of the FOV or crosses into a new block.
  Air-only blocks are not sent beyond `block_send_optimize_distance` (default
  4). `clientiface.cpp` L183-196, L245-262, L333-338.
- **Client mesh queue.** It is FIFO, but "urgent" items (player edits) are
  always taken first. No block is meshed on two threads at once.
  `LUA/src/client/mesh_generator_thread.cpp` L207-235.

### Sodium

- **Task priority score (lower runs first).** `typeValue` (initial build 0,
  rebuild 1, sort 2) + `3·dist/buildDistance` + `0.6·max(dist,50)/50` − 1 point
  per 5 s waiting − 3 if in the frustum. So out-of-frustum work is deferred but
  never starved. `SOD/render/chunk/lists/TaskCollectingTree.java` L15-22,
  L61-81.
- **Which sections get tasks.** Tasks come from the BFS visitors, so occluded
  sections never get meshing work. In-frustum is decided on the LOCAL path.
  `occlusion/OcclusionCuller.java` L291-313.
- **Important (player-edited) updates.** Edits within 16 blocks are flagged
  IMPORTANT and go to the front of the queue. Their defer mode is `ALWAYS` by
  default, with `ONE_FRAME`/`ZERO_FRAMES` blocking the frame on the task.
  `render/chunk/ChunkUpdateTypes.java` L3-10; `RenderSectionManager.java`
  L62-64, L984-996; `gui/SodiumOptions.java` L47.
- **Upload budget.** Uploads may take 30% of frame time, minimum `10_000_000L`
  ns. The source comment says "2ms", but the value is 10 ms. Build and upload
  times are measured to keep the budget estimates current.
  `RenderSectionManager.java` L66-79, L588-613.

### Distant Horizons

- **"Generate low detail LODs first".** The next generation task is the one with
  the minimum `chebyshevDist(center, player) / detailLevel`. A coarser section
  (higher detail-level number) gets a smaller score, so unseen areas fill
  coarse-first. Distance is 2-D only; there is no frustum or travel-direction
  term.
  `DH/core/src/main/java/com/seibel/distanthorizons/core/generation/queues/WorldGenerationQueue.java`
  L296-310.
- **Detail level is a function of horizontal distance.**
  `calcExpectedDetailLevel` uses only XZ distance, divided by the zoom
  magnification inside the camera's zoom cone.
  `core/render/QuadTree/LodQuadTree.java` ~L1205-1240.

### Voxy

- **Requests come from GPU traversal.** The compute shader appends "descend"
  requests for visible nodes that are not frustum- or HiZ-culled. The queue has
  a soft cap (`requestQueueSize`) and a hard cap (`MAX_REQUEST_QUEUE_SIZE`).
  Nodes behind geometry or off-screen never ask for finer data.
  `VOXY/resources/assets/voxy/shaders/lod/hierarchical/traversal_dev.comp`
  L50-63, L119-170.

### Minecraft vanilla / Vintage Story

- Vanilla's compile-queue ordering and the "Chunk Builder" options (Threaded /
  Semi Blocking / Fully Blocking): **unconfirmed** (I found no official
  changelog text). Vintage Story: **unconfirmed** (closed source).

## 3. Never showing holes

- **DH: the parent stays until all 4 children can render.** In
  `onDetailLevelTooLow`, the node is disabled only if all 4 children
  `canRender()`. Otherwise "not all child positions are loaded yet, this one
  should be rendered instead": the parent is enabled and its children disabled.
  The root node is never rendered, to avoid flashing at root boundaries.
  `DH/.../render/QuadTree/LodQuadTree.java` L653-707.
- **DH: zoom is limited to avoid holes.** Zoom-based refinement is disallowed at
  detail levels that may lack child nodes ("Doing so will cause holes/missing
  LODs"). `LodQuadTree.java` ~L1224-1226.
- **DH ↔ vanilla seam.** `overdrawPrevention` (auto by default) sets where LODs
  start. Raising it "may cause holes… most likely… when flying through unloaded
  terrain". `reduceOverdrawWithFastMovement` (default on) pulls the boundary
  closer at speed. Dithered fade (`ditherDhFade`) and `vanillaFadeMode =
  DOUBLE_PASS` blend the two renderers. `DH/core/.../config/Config.java`
  L313-331, L763-788.
- **Voxy: graph invariant plus render-self.** "leaf nodes always contain
  geometry". A node gets children only when a child request `isSatisfied()` (all
  required child meshes are present). While it waits, the traversal enqueues the
  node itself for rendering and adds a request. If a node has no mesh, it
  requests one and descends to its children instead.
  `VOXY/java/me/cortex/voxy/client/core/rendering/hierachical/NodeManager.java`
  L33-37; `NodeChildRequest.java` L83-93; `traversal_dev.comp` L131-170.
- **Luanti: atomic per-block mesh swap.** The old mesh is deleted only when the
  new mesh result is applied on the main thread. If the result is empty, the
  block holds no mesh. Luanti has no LOD system, so there is no coarse fallback.
  `LUA/src/client/client.cpp` L625-666.
- **Sodium.** A section keeps drawing its existing render data until a rebuild
  result is uploaded. There is no LOD layer. (General reading of
  `RenderSectionManager`; no specific "hole" rule found.)

## 4. LOD data on disk

### Distant Horizons

- **SQLite table `FullData`, keyed by `(DetailLevel, PosX, PosZ)`.** Columns
  include `MinY`, `DataChecksum`, `Data`, `ColumnGenerationStep`,
  `ColumnWorldCompressionMode`, `Mapping`, `DataFormatVersion`,
  `CompressionMode`, `ApplyToParent`, `ApplyToChildren`,
  `{North,South,East,West}AdjData` and timestamps. WAL journaling.
  `DH/core/src/main/resources/sqlScripts/0020-…`, `0031-…`, `0080-…`, `0090-…`.
- **Column format, not voxels.** Each source is 64×64 columns
  (`FullDataSourceV2.WIDTH = 64`). Each column holds a top-to-bottom list of
  64-bit data points: 32-bit block/biome id, 12-bit height, 12-bit minY, 4-bit
  sky light, 4-bit block light. Uniform vertical runs therefore cost one entry.
  `core/.../dataObjects/fullData/sources/FullDataSourceV2.java` L77-114;
  `core/.../util/FullDataPointUtil.java` L21-60.
- **Adjacent edge data.** It is stored per row "for significantly reduced render
  loading times". A render cache of meshes "was discovered to not speed up LOD
  loading" and was dropped. `sqlScripts/0090-…`,
  `0040-sqlite-removeRenderCache.sql`.
- **Invalidation.** Per-chunk `ChunkHash`: a chunk is skipped if
  `getBlockBiomeHashCode()` equals the stored hash. Edits propagate up the tree
  through `ApplyToParent` (and down through `ApplyToChildren`). A format bug was
  fixed by deleting `DetailLevel > 0` and re-downsampling.
  `core/.../api/internal/chunkUpdating/ChunkUpdateQueueManager.java` L319-331;
  `sqlScripts/0060-…`, `0100-…`. Detection of a changed world generator:
  **unconfirmed** (no such code found).
- **Generation modes.** `PRE_EXISTING_ONLY(1)` (LODs from existing chunks only),
  `SURFACE(4)` ("does NOT include caves, trees, or structures"), `FEATURES(5)`,
  `INTERNAL_SERVER(6)` (asks the server to generate and save real chunks). They
  are "In order of fastest to slowest".
  `DH/api/.../enums/worldGeneration/EDhApiDistantGeneratorMode.java` L25-65.
  Older BIOME_ONLY-style modes are no longer in this enum.
- **Generating coarse data directly.** Custom generators declare
  `getSmallestDataDetailLevel` / `getLargestDataDetailLevel` and receive a
  `targetDataDetail`. So distant LODs can be produced without generating full
  chunks; the queue splits requests that are too coarse.
  `api/.../worldGenerator/IDhApiWorldGenerator.java`;
  `WorldGenerationQueue.java` L323-361.
- **Cost figures (author-measured, per DTO).** LZ4: read 3.25 ms, ratio 0.45.
  Z_STD: read 9.31 / write 15.13 ms, ratio 0.26. LZMA2: write 70.95 ms, ratio
  0.21, and "Often causes whole computer stuttering". The current default is
  `Z_STD_BLOCK`. `api/.../enums/config/EDhApiDataCompressionMode.java`;
  `core/.../config/Config.java` L1635-1636. Generation-cost figures per mode:
  **unconfirmed**.
- **Server sync is data, not meshes.** The client sends its timestamp for a
  section. The server replies only if `serverTimestamp > clientTimestamp`, and
  login sync and generation requests are rate-limited.
  `core/.../multiplayer/server/FullDataSourceRequestHandler.java` L63-110,
  L153-166.

### Voxy

- **32³ voxel sections at every level, 5 levels (0–4).** Level 4 means "1 chunk
  section is 1 block big". The key is `lvl<<60 | y(8b)<<52 | z(24b)<<28 |
  x(24b)<<4`. Each voxel is a 64-bit `long`: 20-bit block id at bit 27, 9-bit
  biome at bit 47, 8-bit light at bit 56.
  `VOXY/java/me/cortex/voxy/common/world/WorldSection.java` L13-15;
  `WorldEngine.java` L15, L92-108; `common/world/other/Mapper.java` L73-107.
- **Serialized format.** 8 B key, 8 B metadata (LUT size, non-empty-children
  byte), a 32768 × u16 palette index array, then the palette of u64s. Default
  store: RocksDB + ZSTD level 1. `common/world/SaveLoadSystem3.java` L42-82;
  `common/StorageConfigUtil.java` L54-69.
- **Built only from real chunks.** Voxy ingests chunk sections the client
  receives, sections pregenerated with Chunky, and imported worlds. It has no
  terrain generator. Mips take, of the 8 children, the most opaque non-air
  voxel, tie-broken toward the +x+y+z corner.
  `common/world/service/VoxelIngestService.java`;
  `commonImpl/mixin/chunky/MixinFabricWorld.java`;
  `common/world/other/Mipper.java`.
- **Invalidation.** Edits re-ingest the section and `WorldUpdater` re-mips
  parents up to level 4, bubbling up only when emptiness changes. Block-id
  remapping is handled by `SaveLoadSystem3.remap`. Generator-change detection:
  **unconfirmed**. I found no networking code in the repo, so Voxy appears to be
  client-only.

## 5. Rendering

### Voxy (GPU-driven, OpenGL 4.6)

- **8-byte quads.** Layout: face (3 bits), size w/h (4 + 4 bits, up to 16),
  position x/y/z (5 bits each, for a 32³ section), state id (20 bits), biome (9
  bits), light (8 bits).
  `VOXY/resources/assets/voxy/shaders/lod/quad_format.glsl` L1-31.
- **Per-frame pipeline, two-pass occlusion:**
  1. Draw opaque sections using last frame's indirect commands.
  2. Build the HiZ mip chain.
  3. Run the hierarchical traversal (compute: frustum test, HiZ test,
     screen-size `> minSSS` to descend).
  4. Rasterize section bounding boxes against depth, with color and depth writes
     off (`GL_NV_representative_fragment_test` when available), to fill a
     visibility buffer.
  5. Generate commands in compute (`cmdgen.comp`).
  6. Draw with `glMultiDrawElementsIndirectCountARB` and a shared index buffer.
  7. A "temporal" draw adds newly visible sections.

  `java/.../client/core/AbstractRenderPipeline.java` L108-133, L200-203;
  `rendering/section/backend/mdic/MDICSectionRenderer.java` L185-361.
- **Hard requirements.** `glMultiDrawElementsIndirectCountARB` and
  `glDispatchComputeIndirect`. `client/core/gl/Capabilities.java` L60-62.

### Sodium

- **20 bytes per vertex (`CompactChunkVertex.STRIDE = 20`).** Position RG32_UINT
  (20 bits per axis, split hi/lo), color RGBA8 (AO baked in), texcoord RG16
  (relative to the quad's centroid), light/material/section RGBA8_UINT.
  `SOD/render/chunk/vertex/format/impl/CompactChunkVertex.java` L12-20.
- **Regions of 8×4×8 = 256 sections** share buffers. Each region is one
  multi-draw (one command per visible facing slice). Backends: GL
  `multiDrawIndexed`, Vulkan `ext_multidraw`, or indirect.
  `render/chunk/region/RenderRegion.java` L33-35; `DefaultChunkRenderer.java`
  L104-108, L194-300; `gpu/device/backend/DrawBackend.java`.

### Distant Horizons

- **16-byte vertex** (`ushort x,y,z; ushort meta; rgba8; material; normal;
  ushort texture`). There is also a 20-byte variant with sub-block offsets; its
  Javadoc says "24 bytes" but the enum value is 20.
  `DH/api/.../enums/rendering/EDhApiTerrainVertexFormat.java` L21-60.
- **No multi-draw.** One `drawIndexed` per VBO per LOD section, with a
  per-section uniform block. Buffers are CPU-frustum-culled and sorted
  near-to-far, larger sections in front of smaller neighbours. Cave culling
  (default on, below y = 60) drops unlit underground geometry.
  `DHMC/common/src/main/java/com/seibel/distanthorizons/common/render/blaze/BlazeDhTerrainRenderer.java`
  L340-372; `DH/core/.../render/RenderBufferHandler.java` L93-121; `Config.java`
  L792-811.

### Vintage Story (API only)

- **Pooled meshes, multi-drawn.** `MeshDataPool` holds "400k vertices, 300k
  indices but not more than 900 chunks" and draws through
  `GL.MultiDrawElements`. Chunk LODs are distance bands: LOD2 is used beyond
  about 67% of view distance, which covers about 55% of rendered chunks.
  `VS/Client/MeshPool/MeshDataPool.cs` L10-13, L112, L354-358;
  `VS/Client/Render/FrustumCulling.cs` L83-85, L139-170.

## 6. Uniform chunk skipping

- **Sodium.** `section.hasOnlyAir()` produces `BuiltSectionInfo.EMPTY` with no
  mesh task. The VisGraph early-outs (all opaque, or fewer than 256 opaque) skip
  the flood fill. `SOD/render/chunk/RenderSectionManager.java` L397-399;
  `DirectionalVisGraph.java` L49-66.
- **Luanti.** Single-content blocks are stored as one node (`m_is_mono_block`).
  A mesh task is skipped when every block in its cell `isAir()`. The server
  doesn't send air blocks beyond `d_opt`. `LUA/src/mapblock.h` L229, L504;
  `src/mapblock.cpp` L229-253; `src/client/mesh_generator_thread.cpp` L49-61;
  `src/server/clientiface.cpp` L333-338.
- **Voxy.** Air-only sections without light data are not ingested. Each section
  stores a non-empty-children byte. Empty meshes get `EMPTY_GEOMETRY_ID` and
  take no memory. The mesher works on per-row occupancy bitmasks and only
  acquires the neighbours it needs.
  `VOXY/java/.../common/world/service/VoxelIngestService.java` L40;
  `SaveLoadSystem3.java` L76, L96; `NodeManager.java` L57-58;
  `client/core/rendering/building/RenderDataFactory.java` L222-293.
- **DH.** Vertical runs of identical blocks collapse into one data point (the
  column format). `VISUALLY_EQUAL` compression drops hidden blocks such as ores.
  `EDhApiWorldCompressionMode.java`.
- **Vanilla lighting and generation skips:** **unconfirmed**.

## 7. Platform: Bevy 0.20 + wgpu 30 on Metal

- **Bevy 0.20 was released 2026-10-08.** Its release notes mention Solari
  running on Metal, mesh shaders and WESL shaders. They say nothing new about
  MDI, occlusion culling or bindless. https://bevy.org/news/bevy-0-20/
- **`multi_draw_indirect` is emulated on Metal.** It is no longer a feature flag
  and is always callable, but the Metal HAL loops `draw_count` times, issuing
  one `drawIndexedPrimitives(indirectBuffer:)` per command. The GPU reads the
  arguments; the CPU still encodes N draws. `WGPU/wgpu-hal/src/metal/command.rs`
  L1589-1629.
- **`MULTI_DRAW_INDIRECT_COUNT`: not on Metal.** The docs list "DX12, Vulkan
  1.2+" only. The Metal HAL's `draw_indexed_indirect_count` is an empty
  `//TODO`. `WGPU/wgpu-types/src/features.rs` L870-887;
  `wgpu-hal/src/metal/command.rs` L1651-1670. Bevy checks the feature and falls
  back to plain multi-draw indirect. `BEVY/bevy_render/src/render_phase/mod.rs`
  L1202-1204.
- **`INDIRECT_FIRST_INSTANCE`: supported on Metal "Apple3+ or Mac1+".** It is
  set from `indirect_draw_dispatch`. `features.rs` L1670-1684;
  `wgpu-hal/src/metal/adapter.rs` L1202.
- **Binding arrays (bindless) on Metal.** `TEXTURE_BINDING_ARRAY` (with
  non-uniform indexing and partially bound arrays) needs MSL 3.0, Tier 2
  argument buffers and arrays of textures. `BUFFER_BINDING_ARRAY` needs MSL 2.0
  and Tier 2. Sampler-array limit: 1024 on Apple7/Mac2, 500,000 on Apple9.
  `wgpu-hal/src/metal/adapter.rs` L856-877, L1233-1262.
- **Bevy GPU preprocessing and culling.** The mode is chosen per device. Full
  `Culling` mode needs `INDIRECT_FIRST_INSTANCE | IMMEDIATES`, at least 12
  storage textures and at least 10 storage buffers per stage, and compute. Only
  GL and some Android GPUs are excluded by name.
  `BEVY/bevy_render/src/batching/gpu_preprocessing.rs` L148-166, L1563-1600.
  - Metal's limits are 32 storage textures (macOS/Apple6+) and 29 usable
    buffers, and Bevy requests the adapter's limits by default
    (`renderer/mod.rs` L332). So by the source, Metal should reach `Culling`
    mode. **Unconfirmed at runtime**: check the log for "GPU preprocessing is
    fully supported".
  - Bevy 0.16's notes said Metal had GPU transform "but not multi-draw/GPU
    cull". That table predates the current code.
    https://bevy.org/news/bevy-0-16/
- **`OcclusionCulling` component.** Two-phase occlusion culling: an early depth
  prepass of last frame's visible meshes, then a HiZ pyramid, then a
  bounding-box test, then a late prepass. It requires `DepthPrepass`, is
  incompatible with deferred shading and is off by default.
  `BEVY/bevy_render/src/occlusion_culling/mod.rs` L27-69. The 0.16 notes called
  it experimental, with "known precision issues that can mark meshes as occluded
  even when they're not". https://bevy.org/news/bevy-0-16/
- **What can be batched into one multi-draw.** The `Opaque3dBatchSetKey`
  includes pipeline, draw function, `material_bind_group_index` and the
  mesh-allocator slab ids. Default slabs are 1 MiB minimum, 512 MiB maximum, 256
  MiB large-mesh threshold, 1.5× growth.
  `BEVY/bevy_core_pipeline/src/core_3d/mod.rs` L197-220;
  `bevy_render/src/slab_allocator.rs` L183-195.
- **`MaterialExtension` and bindless.** `ExtendedMaterial` is bindless only if
  both base and extension are bindless; the smaller slot limit wins. A
  non-bindless material gets one bind group per material instance. Bindless
  needs only `TEXTURE_BINDING_ARRAY` unless the material uses buffer arrays; the
  comment says that is so "(e.g. on Metal)".
  `BEVY/bevy_render/src/combined_bind_group.rs` L18-46;
  `bevy_render_macros/src/as_bind_group.rs` L79-84, L975-1000;
  `bevy_render/src/material_bind_groups.rs` L223-229.
- **Does a custom `MaterialExtension` get GPU-driven rendering?**
  - My reading of the 0.20 source: GPU preprocessing is a per-view decision
    (`NoIndirectDrawing`), not per material. A `MaterialExtension` mesh
    therefore goes through the same GPU-preprocessed multi-draw path. Without
    `#[bindless]` it batches only with meshes that share the same material
    instance (bind group) and slab. **Unconfirmed at runtime.**
  - Bevy 0.16's notes said "Materials with custom WGSL shaders will continue to
    use CPU-driven rendering by default" unless they opt in with `#[bindless]`.
    I found no per-material switch in the 0.20 source, so this may be out of
    date. **Unconfirmed.**

---

## Implications for Tesserae

Tesserae's constraints: 32³ chunks at every LOD level, CPU-meshed Bevy
`MaterialExtension`, server-authoritative with clients receiving meshes, Metal
first.

1. **Voxy has the same chunk shape.** It uses 32³ sections at every level,
   8-byte quads with 5-bit positions and 4-bit sizes, and an 8-children mip
   tree. Its graph invariant carries over directly to a CPU-side tree: a leaf
   always has a mesh, and a parent is replaced only when all children's meshes
   are resident. The parent-until-4-children rule in DH is the same idea in 2-D.
   A packed quad format also cuts what the server sends per mesh: 8 B per quad
   versus 64–80 B for 4 × 16–20 B vertices.
2. **Store face connectivity with each chunk mesh.** The server can send it in
   the same packet; Sodium's directional version is tighter. The client runs a
   BFS from the camera chunk over 32³ cells: half as many steps per axis as 16³,
   so about 1/8 of the nodes. This needs no GPU features, so it works on Metal
   regardless of what wgpu exposes. Keep the early-outs (all opaque: no
   connections; mostly air: all connections) so most chunks cost nothing.
3. **Order requests on the server the Luanti/Sodium way.** Use distance shells,
   `fov·0.55` plus apex pull-back padding, a narrower cone plus a forward cone
   when moving fast, and a ~10% turn threshold to restart the scan. Score tasks
   like Sodium: distance + frustum bias + age bonus, so off-screen work is
   deferred, not starved. For unseen distant areas, use DH's `dist / lodLevel`
   so coarse levels fill first.
4. **Run server-side occlusion only in a narrow form.** Luanti's server ray test
   cuts sends by "50-80%". But it needs the server to know the client's camera
   and it hides blocks under noclip. Use it for underground or dense terrain,
   with a client BFS as the second stage. Raytraced culling also stops scaling
   at large mesh cells (Luanti disables it at `cell_size >= 4`).
5. **Skip uniform chunks before meshing and before sending.** Mono/air chunks
   get no mesh task and a 1-byte "empty" or "solid" marker on the wire. Put a
   non-empty-children mask on LOD nodes (as Voxy does), so the tree and the BFS
   skip empty subtrees without loading them.
6. **Expect per-chunk draw cost on Metal.** wgpu 30 has no
   `multi_draw_indirect_count` on Metal, and its `multi_draw_indirect` is a CPU
   loop of indirect draws. Voxy's GPU-generated draw count therefore doesn't
   port. Keep CPU-side visibility (BFS + frustum) authoritative, cut draw count
   by merging chunks into Sodium-style regions in shared buffers, and keep the
   material in one bind group: a single atlas or array texture, `#[bindless]` if
   possible.
7. **Measure Bevy's `OcclusionCulling` before relying on it.** By source, Metal
   should qualify for Bevy 0.20's `Culling` mode. The component is opt-in, needs
   `DepthPrepass` and has documented false-occlusion issues. It could complement
   the BFS for tall surface scenes, but only after A/B timing, and only for
   chunk entities with tight AABBs.
8. **Make swaps atomic, and fall back to the parent.** Keep the old mesh until
   the replacement is uploaded (Luanti, Sodium), and render the parent LOD while
   children are pending (DH, Voxy). Within one level, apply edits atomically by
   version. Across levels, reveal children only as a complete set, so neither
   edits nor refinement ever open a hole.
9. **Cache on the client and sync by timestamp, as DH does.** The client sends
   its cached version or timestamp per node, and the server replies only when
   its copy is newer, under a rate limit. Since Tesserae sends meshes, also send
   the per-chunk face-connectivity bits and the empty/solid flags, so the client
   never needs voxels to cull.
10. **Choose a vertex budget early.** Sodium uses 20 B per vertex, DH 16 B per
    vertex, Voxy 8 B per quad. A Bevy `Mesh` with standard attributes is much
    larger, so a packed custom vertex attribute in the `MaterialExtension`
    (decoded in the vertex shader) is likely the biggest cheap win for both
    upload and network.
