# The dev gallery

Status: done
Blocked by: 06

Nothing in this step is observable on terrain, which has no shaped Voxel (spec
decision 11), and the `dev` capture cannot click. The step-4 lightbox is the
precedent: a `dev`-only structure placed near the spawn point. This one goes
through the real edit path.

## What to build

`server/gallery.rs`, `dev` only, a sibling of `lightbox.rs`:

- Placed by a metre position ahead of the spawn point and to one side of the
  lightbox, so the default camera frames it; the pattern itself is a count of
  Voxels (like `testgen`) because the point is the shape of individual Voxels,
  and a test says so.
- A plinth of cubes to stand on, then: a catalogue row (cube, bottom and top
  slab, a stair in each of four turns, an upside-down stair, a wall-mounted
  slab, a post); a flight of stairs up to a slab platform, so the occlusion
  between stairs and cubes and the culling of a slab floor are both on screen;
  and a lantern on the platform, so block light falls on shaped faces.
- Placed a few Voxels per tick through the shared Server-internal entry, after
  the Chunks it needs are loaded and lit, as the lightbox waits.
- Extract whatever the two builders share (the plan runner and its wait) rather
  than copy it.

## Tests

- The plan stays under the shaped cap per Chunk, uses every built-in Shape, and
  uses more than one Orientation of the stair.
- The plan's footprint in metres is what it claims.
- Every Voxel in the plan is valid (registered Shape, valid Orientation).

## Comments

The plan runner both builders share is `server/builder.rs`; the lightbox was
moved onto it rather than duplicated. Run with
`TESSERAE_SCREENSHOT=out.png TESSERAE_SCREENSHOT_FLY_S=0 TESSERAE_SCREENSHOT_SETTLE_S=10 cargo dev`
and the default camera frames the gallery to the right of the lightbox. 679
Voxels, 42 of them shaped, placed in a few ticks; the steady state is unchanged
at 2167 Chunks loaded and 791 meshed.

The first capture after a fresh build came out pure black and the identical
command a few seconds later was fine, so it was the window not being presented,
not the scene.
