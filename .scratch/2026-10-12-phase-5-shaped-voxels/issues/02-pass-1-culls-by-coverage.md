# Pass 1 culls by coverage

Status: done
Blocked by: 01

Today the greedy pass asks `is_full_cube()` of the Voxel across a face. That
becomes a per-face question, which is the change most likely to put a hole in
the world, so it lands on its own before any shaped geometry exists to hide a
mistake behind. Spec decision 4.

## What to build

- Thread `&ShapeTable` through `greedy_mesh` and `mesh_chunk`. The emitter test
  stays `is_full_cube()`, because shape 0 is the only thing pass 1 merges. The
  occlusion test becomes `shapes.seals(neighbour, opposite(face))`.
- `Chunk::may_contain_shaped()`: any palette entry that is a non-air Voxel with
  a non-zero shape, or the uniform Voxel. A palette never demotes, so it can
  only err towards `true`. And `Chunk::shaped_count()`, which scans only when
  the first is true.
- `Chunk::filled` and `Chunk::air` become `const fn`, so tests, benches and the
  Server can lend a `static` air or solid Chunk.
- `Neighborhood` records whether the centre may contain shaped Voxels, for
  ticket 03.
- Update every call site, in tests and benches, with a shared helper rather than
  a table per test.

Benchmark `greedy_mesh/terrain`, `/solid` and `/checkerboard` before and after.
The inner loop got a table lookup in the cube-and-air case it did not have; if
it moves, find out why before rewriting it, and record the number.

## Tests

- A cube beside a bottom slab, on every side: the cube's face is culled only on
  the sides where the slab seals.
- A cube above a bottom slab is culled on its -Y face; a cube above a top slab
  is not.
- A cube beside a stair keeps its face on the stair's open sides and loses it on
  the sealed ones.
- Every existing meshing test passes unchanged in meaning; the three that used
  `Voxel::new(1, 1, 0)` as "some shaped Voxel" are rewritten for the real slab
  (they asserted that shaped Voxels emit nothing, which stops being true in
  ticket 03).
- `may_contain_shaped` and `shaped_count` across uniform, palette, and an
  edited-then-restored Chunk (the false positive is allowed and pinned).

## Comments

Asking `seals` of every neighbour measured 2-6% slower on `greedy_mesh/solid`
and `/terrain` against the step-4 baseline. Testing `is_full_cube()` first and
reaching the table only for a shaped neighbour recovered all of it: terrain
-3.5%, solid +0.3%, mesh_chunk/straddling -3.3%. The comment in `greedy_mesh`
records this.

The `Shapes` resource, which ticket 06 listed, landed here instead: the Server's
`publish` needed a table the moment `mesh_chunk` took one.

`Neighborhood` does not record the may-contain-shaped flag yet; ticket 03 is its
only reader and an unread field would fail `-D warnings`.
