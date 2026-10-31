# A level is drawn until another level replaces it

Status: accepted.

## Context

Every LOD level draws only part of the ground, and the Server streams each
level on its own: it unloads a coarse Chunk once it falls inside the finer
level's hole, whether or not the finer Chunks exist yet, and a level-0 Chunk
once it leaves level 0's ring, whether or not level 1 has meshed there. The
Client then cut every coarse fragment within a fixed metre radius of the Eye,
delivered or not. Diving, climbing and teleporting showed the clear colour
wherever one level had let go before the next arrived. See
`.scratch/2026-10-29-streaming-performance/`.

## Decision

**A level-k fragment is discarded wherever a finer level draws the Chunk at
that point, and nowhere else.** Drawing a Chunk includes an empty mesh, which is
the finer level's answer that the space is air. The Client keeps one bitset per
level of the Chunks it draws (`client::drawn_mask`), and all three terrain
materials read it.

**The Client retains what the Server unloads until it is covered.** A Chunk the
Server unloads stays drawn until another level's delivered meshes cover its
region: a coarse Chunk once each of its eight children is delivered or, in turn,
covered by its own children; a finer Chunk once a coarser ancestor is delivered.
It is also released once it lies outside every level's mesh extent around the
camera. A release despawns the entities and clears the bit in the same frame.

Retention is Client presentation only. The Server's rings, unloads and World
state are unchanged and the Client still receives meshes and nothing else
([ADR-0002](0002-client-server-from-day-one.md)). The metre cut and the coarse
cameras' raised near plane are gone.

Voxy keeps the invariant "leaf nodes always contain geometry" and Distant
Horizons disables a parent only once its four children can render; this is the
same rule per Chunk, so a delivered child replaces its part of the parent at
once instead of all children together.

## Consequences

- **A blocky patch replaces a hole.** Until the finer level arrives, a coarse
  Chunk is drawn where the finer one will be, up to a coarse Voxel above the
  real surface.
- **Retained meshes cost memory and draws** until their region is covered. They
  are bounded by the coarsest level's mesh extent; a teleport releases the old
  ones as the new coarse levels arrive.
- **One camera and one depth buffer draw every level.** Because the coverage
  rule already discards a coarse fragment wherever a finer level draws, depth
  decides the rest, and the five coarse cameras, each clearing its own depth and
  overdrawing the one before, are gone. Level 0 is the only shadow caster. The
  one visual change: a coarse Chunk not yet refined can now hide finer terrain
  behind it, as any nearer surface would.
