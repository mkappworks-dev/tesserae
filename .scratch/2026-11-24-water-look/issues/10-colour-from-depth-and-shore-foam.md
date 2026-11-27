# 10: Colour from depth and shore foam

Status: done
Blocked by: 06, 08

Spec: [Water that looks natural](../spec.md), The water path and Water columns.

## What to build

Shallows show the sand and deep water turns darker and bluer. The Client uploads
each layer's water columns into its range of the storage buffer, after its
records, with the header naming where they start. A merged quad spans many
columns, so the fragment finds its column from the quad's first Voxel plus the
floor of its UV; the vertex stage forwards only the buffer offset. Depth is
filtered across columns. Each channel keeps `exp(-absorption × path)` of the
floor, where path is depth over the cosine of the view angle; the scatter tint
replaces what is lost; alpha follows the mean transmittance. A foam band, broken
by the engine's foam mask, lines water shallower than a few tenths of a metre,
and ripples calm in the shallows.

## Acceptance criteria

- [x] A chunk mesh test: the header names where the water columns start and they
      upload whole; a dry layer uploads none.
- [ ] Lake and coast captures: clear shallows, deep water darker and bluer with
      depth, a foam line at the shore; shown to the user.
- [x] No colour step where two Chunks of one lake meet, corners included.

## Comments

- 2026-11-27, on 089a21cf plus this ticket:
  - A wet blended layer carries its water columns after its records, one word
    each (`WaterColumn`'s four bytes), and a fifth header word names where they
    start, 0 for none; coarse layers and dry Chunks carry none. The vertex
    stage forwards that word's address, flat.
  - The fragment finds its place from the quad's first Voxel, wrapped to the
    Chunk, plus the face's UV. A first version stepped Voxel by Voxel as the
    tile turn does; at a quad's far edge it stepped into the next Chunk,
    wrapped to this one's near side, and drew a thin line along every Chunk
    edge in a top-down lake view.
  - Depth is filtered bilinearly between column centres, over the apron, so
    two Chunks of one lake meet with no step. A dry column in the footprint
    stands as far below as the wet ones are above, so the depth runs out to
    nothing at the waterline: without it the shore stopped half a column short,
    about 0.5 m deep, and no foam showed.
  - Each channel keeps `exp(-absorption × depth / cos)` of what lies behind;
    the lost share becomes the scatter tint, as albedo bevy_pbr lights, and
    alpha is the mean lost share. From below, and at coarse levels, water keeps
    08's fixed colour.
  - Shore foam: shallower than 0.6 m, a band whose inner edge the engine's
    foam mask (128², R8, every mip, a sum of integer-crest sines) breaks up;
    foam is rough and takes no reflection. Ripples calm over the last metre.
- Captures in `/tmp/tesserae-water-look-captures/10-after/`: lake (the floor
  through the shallows, the middle darker), `lake-above` (45 m up, pitch -55,
  over the lake's Chunk edges, no step), coast (a foam line along the beach)
  and estuary. With pure water's absorption, shallow river channels seen from
  above are nearly clear; a Game's own look (ticket 15) is what colours them.
  They wait on the user's review.
