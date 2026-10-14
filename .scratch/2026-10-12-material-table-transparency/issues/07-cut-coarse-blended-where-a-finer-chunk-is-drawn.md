# Cut coarse water where the finer level draws

Status: ready-for-agent
Blocked by: 06

Ticket 06 accepted a band of doubled water at each LOD ring join: between the
finer level's guaranteed reach and where it actually ends, both levels draw
water and the finer composites over the coarse. A fade was tried and was worse.
The exact rule is that a coarse blended fragment is redundant wherever the
finer level has delivered the Chunk that contains it, whatever that Chunk holds:
water, ground or nothing.

## What to build

- The Client tracks, per level, the Chunks it has been sent a mesh for and not
  told to unload, empty meshes included (an empty mesh is still the finer
  level's answer for that space).
- For each coarse level k, the blended material carries a bitmask of level
  k-1's delivered Chunks over a window around them, and the window's origin.
  Updated when the set changes.
- The chunk shader discards a level-k blended fragment whose level-(k-1) Chunk
  is in the mask. The opaque material carries no mask and is unchanged.

## Acceptance criteria

- Tests: the mask holds exactly the delivered Chunks, an unload clears its bit,
  an empty mesh sets it, a Chunk outside the window is reported as not drawn,
  and the opaque material's mask is empty.
- Capture of the ring join on a flooded World: the doubled band is gone (water
  luminance across the join within the lake's own variation), and no dry gap
  appears where the finer level stops.
- The opaque scene is pixel-identical to `main` (`TESSERAE_TRANSLUCENT_ROW=0`).
- `./scripts/cargo-ci` passes.

## Comments
