# Per-face light opacity in both solvers

Status: done
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

## Comments

**Mutation checks found a hole in the tests.** Four mutations were applied to the
solvers: ignoring the neighbour's exit seal in `seed_from_border`, testing only
the destination in the bulk `flood`, dropping the `may_contain_shaped` guard on
the open-sky shortcut, and ignoring the source's seal in `brighten`. Three were
caught at once. Ignoring the source's seal in the bulk flood survived, because
the seam test only crosses a sealed face at a Chunk edge, where the flood never
asks. `the_two_solvers_agree_on_shaped_scenes_inside_one_chunk` was added for
it and now kills it.

**The flood is the crate's hottest loop and the optimiser decides its speed.**
Putting the general edge rule in it doubled `light_chunk/straddling`
(85 us to 171 us). Specialising on `SHAPED` recovered some; forcing `inside` and
`transmitted` inline recovered nothing; making the channel a const generic as
well recovered all of it. With both constant, `flooded_air` is at baseline
(-1% to +1%). The cause was the same one `inside`'s comment records from step
4: the baseline relied on the optimiser unrolling the two-channel loop so that
`transmitted`'s sky case folded away for block light, and a second copy of the
function stopped that. The channel loop is now spelled out.

What is left, measured twice: `straddling` +6-10% (per-Voxel mask instead of a
bool, and a 96-byte border passed by value instead of 48), `straddling_settled`
+3%, `open_sky` and `opaque` +0.7 ns of a few ns. About 0.1 ms on a 15.6 ms
tick at the budget. A try at specialising the mask loop for Chunks with no shaped
entry measured worse, including on benches it did not touch (3.5 to 4.0 ns), so
differences under about 10% on these benches are code layout and the change was
reverted.

A Chunk with a stair on every column takes 139 us against 91 us for terrain
(`light_chunk/straddling_shaped`).
