# Spawn the 3x3 Chunk grid, a fly camera and a directional light

Status: done
Blocked by: 03

## Task

Spawn `testgen::terrain_chunk(cx, 0, cz)` for a 3x3 grid as `Mesh3d` entities,
each translated by `chunk_coord * CHUNK_SIZE * voxel_size` with
`voxel_size = 1.0`.

Build each Chunk's `Neighborhood` with the neighbours that exist in the grid,
not `Neighborhood::isolated`. Isolated chunks emit a full skin of faces at
every boundary — invisible, because the faces are buried in solid rock, but it
is the dominant cost in a streamed world and the grid is the first chance to
exercise the API step 3 needs.

Add a fly camera — WASD, mouse look, shift to sprint, Esc to release the
cursor — hand-rolled, not a dependency. Camera speed is metres per second.

Add a directional light with shadows enabled so the shadow maps have something
to do, and leave Bevy's default tonemapping in place.

## Done when

`cargo run -p tesserae` opens a window on nine Chunks of lit terrain that a
human can fly around. No seams between Chunks, no inside-out faces, no
stretched UVs.

## Notes

The grid is a fixed scene, deliberately not a streamer: step 3 replaces it.
Keep it in its own module so the replacement is a deletion.

Generating Chunks is World generation, which the Server owns from step 3 on. A
startup system that fills a resource and never writes it again does not
contradict [ADR-0002](../../../docs/adr/0002-client-server-from-day-one.md);
a Client system that mutates a Chunk would.

## Comments

A flat ambient fill was added alongside the directional light. Without it every
face the sun does not reach is black, and vertical faces are the only place
Dirt and Stone appear at all — the chunk material would have been unverifiable
by eye. Step 4's flood-fill replaces it.
