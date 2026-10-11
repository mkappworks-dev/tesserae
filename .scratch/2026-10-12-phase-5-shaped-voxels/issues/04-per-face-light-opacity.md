# Per-face light opacity in both solvers

Status: ready-for-agent
Blocked by: 01

Replaces `blocks_light`. Spec decision 5. The two solvers must keep agreeing,
and agreeing across a Chunk boundary is the new part.

## What to build

- Delete `blocks_light`. One shared edge rule, `light_passes(shapes, from, into,
  face)`: blocked if `into` seals its face towards `from`, or if `from` is
  shaped and seals its face towards `into`. A full cube is not asked the second
  question.
- `light_chunk(center, border: [Option<LitChunk>; 6], shapes)`. `opaque:
  Vec<bool>` becomes `Vec<u8>` from `ShapeTable::light_mask`; `flood` tests the
  source's exit bit (hoisted out of the face loop) and the destination's entry
  bit; `seed_from_border` tests the inside cell's entry bit and, behind
  `neighbour.chunk.may_contain_shaped()`, the neighbour Voxel's exit bit.
- The open-sky fast path also requires that the +Y neighbour cannot contain a
  shaped Voxel; add the comment explaining why. The opaque fast path is
  unchanged.
- `relight(volume, shapes, edited, was)`: `brighten` uses `light_passes`.
  `darken` keeps traversing every edge, with a comment saying why that is
  correct and not an oversight.
- `streaming::relight` passes the neighbours' Chunks as well as their light. The
  synthetic open-sky border above the surface is an air Chunk with full sky.

Benchmark `light_chunk/straddling`, `/straddling_settled`, `/flooded_air`,
`/open_sky`, `/opaque` before and after. The flood is the hot loop and
`inside()`'s comment records what an innocent rewrite of it cost in step 4; the
seal test goes in beside it, not instead.

## Tests

- A stair roof over a column: sky is blocked by its sealed -Y face and reaches
  the floor only from the sides; a top slab does the same; a bottom slab lets
  sky into its own cell and not below it.
- Light enters a stair's notch through its open side and leaves through the
  other open faces.
- A lantern (a sealed cube) still lights its surroundings.
- **The two solvers agree** on scenes with shaped Voxels, extended to a
  two-Chunk lattice with the shape standing **on the seam**: the same scene
  built wholly inside one Chunk and straddling two must give identical light
  everywhere, through `light_chunk` to a fixed point and through `relight` one
  edit at a time.
- Removing a stair returns every Voxel to its previous value exactly.
- The joint-coverage leak (top slab beside bottom slab) is pinned as known
  behaviour.
- The open-sky fast path agrees with the flood when the Chunk above holds a
  bottom slab.
