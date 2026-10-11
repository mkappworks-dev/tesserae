# The Client's chunk view, and the end of scene.rs

Status: ready-for-agent
Blocked by: 04

`crates/tesserae/src/scene.rs` spawns a fixed 3x3 grid at `Startup`. Its module
doc says step 3 deletes it and that nothing else depends on it. This ticket
replaces it with a Client that renders whatever the Server sends.

## Task

### `crates/tesserae/src/client/chunk_view.rs`

- `ClientChunks`, a `Resource` mapping `ChunkCoord` to the `Entity` holding its
  mesh. Client-owned: these are render entities, not World state.
- One `Update` system reading `ChunkMeshReady`. For an empty mesh, despawn any
  entity for that coordinate and drop the map entry — one rule, matching the
  Server sending empty meshes rather than suppressing them. Otherwise convert
  with `chunk_mesh_to_mesh` and `world.voxel_size`, which is still the single
  place Voxel-index space becomes metres, and spawn or replace the entity.
  Replacing means removing the old `Mesh` asset handle so the asset is dropped;
  streaming for a few minutes otherwise leaks every mesh ever uploaded.
- One `Update` system reading `ChunkUnloaded` that despawns.
- The shared `ChunkMaterial` handle, added once at `Startup`. One material for
  every Chunk, as `scene.rs` did.

### Camera and light move out of `scene.rs`

- The light and the ambient fill move to `client/`. Keep step 2's comment about
  the fill standing in for sky light until step 4's flood-fill.
- The camera spawns in `PostStartup` so the Server's `Startup` system has set
  `SpawnPoint`, with no ordering edge between the two plugins to maintain. Set
  `CameraHome` from it — `camera.rs` already says the Server's spawn point
  should own it from step 3 — and add `StreamingAnchor`.
- Delete `scene.rs` and its `mod scene;` and plugin registration in `main.rs`.

## Tests

The spawn-and-despawn systems need an `App`, so drive them with a minimal
`App` plus `MinimalPlugins` and `AssetPlugin` rather than `DefaultPlugins`,
which wants a window.

- A `ChunkMeshReady` with geometry spawns one entity carrying `Mesh3d` and the
  coordinate; a second message for the same coordinate replaces it rather than
  spawning a second, and the old `Mesh` handle is gone from `Assets<Mesh>`.
- An empty `ChunkMeshReady` despawns an existing entity.
- `ChunkUnloaded` despawns and clears the map entry.
- Nothing in `crates/tesserae/src/client/` names a `ChunkStore` mutator. A grep
  in a test is not worth it; the compiler is the check, so just confirm the
  build.

## Done when

`cargo dev` streams Chunks in and out around the moving camera, R returns to
the Server's spawn point, `scene.rs` does not exist, `cargo test --workspace`
and `cargo lint` are clean.

## Comments
