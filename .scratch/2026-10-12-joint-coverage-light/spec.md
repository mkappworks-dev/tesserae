# Joint coverage in the light edge rule

Closes the loose end that decision 5 of
[the shaped-voxels spec](../2026-10-12-phase-5-shaped-voxels/spec.md) pinned
with a test: two partial Shapes that only **jointly** cover a face (a bottom
slab beside a top slab) each leave the face unsealed, so light leaks through the
seam between them.

## Decision: do it, and let only shaped Chunks pay

The leak is a lighting bug a player can see: a wall of alternating slabs, or a
stair meeting a slab, lets light through where the geometry shows none. The
mesher already culls by **joint** coverage (decision 4 asks whether a rectangle
is a subset of the neighbour's mask), so the light and the picture disagree
about the same wall.

It is worth doing because the cost can be confined. Terrain holds no shaped
Voxel (`shaped_count() == 0` is a worldgen contract), so `light_chunk` runs its
`SHAPED = false` instantiation on it, and that instantiation is not touched:
same function, same argument list for the optimiser to see through, same
`blocked[ni] == 0` test. Only `flood::<true, _>`, `seed_from_border` behind its
`may_contain_shaped` gate, and `relight` pay. The old reason for not doing this
("doubles the inner loop") assumed paying on every edge; it is not necessary.

If the benchmark after shows `SHAPED = false` moved, the change is reworked, not
accepted (see Performance).

## The rule

Light leaves Voxel A through face `f` into Voxel B, entering through `g = f ^ 1`.
Let `L` be A's coverage mask on `f` if A is shaped and empty otherwise, and `E`
be B's coverage mask on `g`. The edge is **blocked iff `L | E` covers the whole
plane**, and open otherwise.

- A full cube as A contributes an empty `L`, not a full one. That is the step-5
  asymmetry (a cube only holds its own emission, and leaves freely) and what lets
  a lantern light a room; it is kept, not new.
- Everything the old rule blocked is still blocked: if `E` is full, or A is
  shaped and `L` is full, the union is full.
- What is new: both sides **partial** on the facing faces (neither empty nor
  full) and their union full. A bottom slab's +X face is the lower half, a top
  slab's -X face is the upper half, and together they are the plane.
- `FaceMask`'s convention already makes a Voxel's face and the matching face of
  the Voxel across it index the same squares (the mesher's occlusion test relies
  on it), so the union is a plain row-wise OR with no flipping.

## How it stays off the hot path

The existing per-cell `u8` (`light_mask`: six seal bits and `LIGHT_EXIT_CHECKED`)
is **unchanged** and still decides every edge it can decide. The joint test is
reached only when it would call the edge open **and** both cells are partial.

- `ShapeTable::joint_id(voxel) -> u16`: non-zero when some face of the Voxel is
  neither empty nor full, and then a handle (variant slot + 1) for
  `ShapeTable::jointly_seals(from, face, into)`. A cube, air and an unknown Shape
  are zero, which is what keeps a lantern's exit free without a special case.
  A handle rather than the Voxel, so the flood can hold one per cell in two bytes
  and the cold path is two table loads and a row compare, with no palette read.
- `FaceMask::jointly_full(&self, other)`: row-wise OR, early out on the first
  row that is not all ones. Most partial pairs (an L beside an L) fail on row 0.
- `light_chunk` builds a per-cell `joint: Vec<u16>` in its existing sweep, **only
  when the Chunk may contain a shaped Voxel**. `blocked` keeps its type and its
  meaning, so the unshaped flood reads exactly what it read before.
- In `flood::<true, _>` the test follows `edge_open`: both ids non-zero, then
  `jointly_seals`. `flood::<false, _>` is not edited. The two solvers share the
  rule through `ShapeTable::jointly_seals`, and `light_passes(shapes, from, into,
  face)`, which `relight`'s brighten pass calls, is `edge_open` plus the same.
- `seed_from_border` needs the neighbour's Voxel across the plane (already read
  behind `may_contain_shaped`); its joint id is looked up only when the inside
  cell's is non-zero. The joint rule therefore applies across a Chunk seam
  exactly as it does inside one, and the incremental solver, which sees both
  Voxels, agrees.

`darken` is unchanged and its comment stands: it traverses every edge, and
over-clearing is repaired by regrowth, which does follow the joint rule. An edit
to B changes only edges incident to B, and `relight` already re-examines B's six
neighbours, so no new invalidation is needed.

## Invariants kept

1. `tesserae-core` stays free of Bevy and wgpu; nothing here is physical, so
   ADR-0003 is untouched (a coverage mask is dimensionless).
2. The two solvers agree: bulk to a fixed point and edit by edit, inside a Chunk
   and across a seam. The existing agreement tests gain joint scenes.
3. `Light` storage and the `ChunkLight` API do not change, so nothing downstream
   (mesher, Server) moves.

## Tests

Each new test is mutation-checked: the code is broken on purpose and the test
must fail.

- `FaceMask::jointly_full` and `light_partial`, from the built-ins.
- The pinned test is **flipped**, not deleted: bottom slab beside top slab in a
  tunnel now stops the light. Its siblings: a stair's L beside the opposite L
  (a stair beside a stair that interlock), partial beside a **cube** source
  (still open), partial beside air (open), two bottom slabs side by side (the
  upper half is open, so light still passes: not every partial pair seals).
- The agreement tests (inside one Chunk, and on the seam) gain the joint
  scenes, on both sides of the seam and in both channels.
- Removing either half restores every Voxel to where it was.

## Performance

Baseline before any code:
`cargo bench -p tesserae-core --bench meshing -- --save-baseline before`, and the
same with `--baseline before` after. The numbers to watch, in order:

1. `light_chunk/straddling`, `/straddling_settled`, `/flooded_air`, `/open_sky`,
   `/opaque`, `/cave_*`: **must not move beyond noise**. These run
   `SHAPED = false`. A change here is code layout and means the instantiation
   was disturbed, so it is reworked.
2. `light_chunk/straddling_shaped`: may rise, and the PR says by how much. It is
   a stair on every column, so most partial pairs are stairs sharing a side:
   the worst realistic case for the cold branch.
3. `snapshot/*`, `greedy_mesh/*`, `mesh_chunk/*`: untouched code, a check on the
   noise floor of the run.

A micro-optimisation (inline attributes, hoisting, branch layout) goes in only
if its own before/after bench says so.

## Out of scope

- Three-way joint coverage. The rule is pairwise because an edge has two sides.
- Light that exits a Voxel by a face and re-enters by a neighbouring one inside
  the same cell; that is the existing rule, per face.
- Any change to what the mesher culls.

## Done when

`./scripts/cargo-ci` passes; the pinned test is flipped and DESIGN.md's loose
end and decision 5 of the step-5 spec say so; the before/after table is in the
PR; every new test has been seen failing under a mutation.
