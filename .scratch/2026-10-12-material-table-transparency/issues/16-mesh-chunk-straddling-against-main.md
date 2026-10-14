# `mesh_chunk/straddling` reads 4 to 5% slower than `main`

Status: done
Blocked by: 09

With one codegen unit on both sides, `mesh_chunk/straddling` measured 417.8 us
on the branch and 397.8 us on `main`, with `main` itself spreading to 565 us in
the same runs. Nothing on a dry straddling Chunk's path changed in kind.

## What to build

- Interleaved runs, one codegen unit, at enough repetitions to tell a real
  difference from the spread, and a bisect across the branch's core commits if
  it is real.
- If a commit is the cause, fix it or record why it is the cost of what it
  bought. If it is not real, say so with the numbers.

## Comments

Investigated; the cost is ticket 10's and is not reworked.

- **Bisect**, `CARGO_PROFILE_BENCH_CODEGEN_UNITS=1`, each commit built once in
  its own worktree, six interleaved rounds at load 2.8 to 5.0, medians:
  `origin/main` 408.6 us, ticket 11 (2e8f19d) 400.2, ticket 10 (f903730)
  417.9, ticket 09 (625d91c) 420.2, ticket 15 (8fb3149) 422.0. The step is at
  ticket 10, about 4%, and steady from there.
- **Where.** Between tickets 11 and 10, three rounds: the benches with shaped
  Voxels are 5 to 9% slower (`ramp_densest` 432 to 472 us, `all_stairs` 3.47 to
  3.68 ms, `terrain_stairs_at_cap` 576 to 608 us), `greedy_mesh/terrain` is 3%
  faster, and `straddling_unsmoothed`, which runs no pass 2, read 5% slower in
  one set and level (342 against 339 us) in the next.
- **What that is.** Ticket 10 adds a third `MeshLayer` and two more bits to the
  pass-2 merge key; nothing on a dry Chunk's path changed in kind. A control,
  ticket 11 with only an unused third `MeshLayer` added to `ChunkMesh`, ran
  `straddling_unsmoothed` at 363 us against ticket 11's own 339 and ticket 10's
  342 in the same rounds: a struct that is never written moves this benchmark
  by 7%. That is the code-layout sensitivity the LOD step met (ticket 09 of its
  spec), and it is larger than the step being explained.
- **Decision.** About 4% on a Chunk with shaped Voxels, 15 to 20 us of a
  meshing Job off the main thread, is accepted as the cost of the cutout pass. A
  profile (`xctrace` is on the machine) is the next step if it ever matters.
