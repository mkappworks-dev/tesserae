# Cut coarse water where the finer level draws

Status: done
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

Built as specified, with the mask in a storage buffer (`ShaderBuffer`) rather
than a uniform: its window is the bounding box of what each level has
delivered, so it fits any Voxel size and any streaming settings. Layout in
`client/drawn_mask.rs`, mirrored by `finer_drawn` in the shader. The opaque
material's buffer is one zero word and cuts nothing.

- Tests: the mask holds exactly the delivered Chunks, an empty mesh sets its
  bit, a Chunk inside the window that was not sent and one outside it are not
  cut, an unload clears the bit, level 2 asks level 1, and the opaque material
  cuts nothing. The tests now run the app's own `Update` chain
  (`add_view_systems`): a first version wired `publish_drawn` into the tests'
  copy of the chain and not the app's, and the capture is what caught it.
- Capture, World flooded to 44 m, horizon view, 30 s settle: the cut changes
  6223 pixels, 98% of them water, all in the band along the ring join; on
  average they get lighter (a doubled layer removed), and some darker where
  the finer level's own surface sits lower than the coarse one's. What is left
  in that band is the coarse lake bed seen through the finer water, lit as open
  sky (coarse levels store no light under water), not a second layer.
- No crack: the magenta probe finds the same 3 silhouette pixels as before.
- Against `origin/main` at the merge (9ec5571, built in its own worktree), with
  `TESSERAE_TRANSLUCENT_ROW=0`: the only differences are the frame-rate digits
  and the hotbar hint in the overlay text.
- Limit: a coarse water surface above the finer level's top row, which only
  happens near the vertical edge of a level, is not cut.

