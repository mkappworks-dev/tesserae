# 10: Colour from depth and shore foam

Status: ready-for-agent
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

- [ ] A chunk mesh test: the header names where the water columns start and they
      upload whole; a dry layer uploads none.
- [ ] Lake and coast captures: clear shallows, deep water darker and bluer with
      depth, a foam line at the shore; shown to the user.
- [ ] No colour step where two Chunks of one lake meet, corners included.
