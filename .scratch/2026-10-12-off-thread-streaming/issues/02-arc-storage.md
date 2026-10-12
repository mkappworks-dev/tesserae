# ChunkStore holds Chunks and light behind Arc

Status: done
Blocked by: none

A pure refactor with no behaviour change, so that when a worker holds a Snapshot
the cost of that is already known. Spec decision 3.

## What to build

In `server/world_state.rs`:

- The `chunks` and `light` maps hold `Arc<Chunk>` and `Arc<ChunkLight>`.
- `get` and `light` still return `&Chunk` and `&ChunkLight`. No read site
  changes.
- `insert` wraps; `set_light` wraps; `set_voxel` and `LightLattice::set_light`
  mutate through `Arc::make_mut`.
- A `pub(in crate::server)` accessor returning the `Arc`s for a coord. It is the
  only way an `Arc` leaves the store, and it is Server-private for the reason in
  the module doc: an `Arc<Chunk>` in Client hands is read access.

Add a core-or-tesserae microbenchmark (criterion in `tesserae-core/benches` can
clone the types; the `Arc` side is `std`) of, for a surface-crossing Chunk and
its dense light: a full clone of seven Chunks and seven `ChunkLight`s, against
fourteen `Arc` clones, against `Arc::make_mut` on a shared and an unshared
Chunk. The spec rejects the full clone on an estimate; this puts the number in
the PR.

## Acceptance criteria

- The whole existing test suite passes unmodified.
- A test that an edit to a Chunk whose `Arc` is held elsewhere does not change
  what the holder sees, and that an edit to an unshared Chunk does not allocate
  (pointer equality of the `Arc` before and after).
- The mutators are still `pub(in crate::server)`; `grep` for
  `ResMut<ChunkStore>` outside `crate::server` finds nothing new.
- Benchmark numbers in Comments, and the spec's "roughly 300 KiB" claim is
  confirmed or corrected.
- `./scripts/cargo-ci` passes.

## Comments

Benchmarks (`cargo bench -p tesserae-core -- snapshot`, Apple M5), for a
surface-crossing Chunk with its dense light:

| Case                                          | Cost     |
| --------------------------------------------- | -------- |
| clone seven Chunks and seven `ChunkLight`s    | 5.6 us   |
| clone fourteen `Arc`s                         | 43 ns    |
| `Arc::make_mut` on an unshared `Chunk`        | 3.3 ns   |
| `Arc::make_mut` on a shared `Chunk` (clones)  | 152 ns   |
| `Arc::make_mut` on a shared dense `ChunkLight`| 661 ns   |

The spec's estimate was wrong by two orders of magnitude and has been corrected:
the full copy is 5.6 us a mesh Job, not hundreds. It is dominated by the 224 KiB
of dense light; terrain Chunks are a few KiB. `Arc` still wins by 130x and makes
`Arc::ptr_eq` the "unchanged since the snapshot" test, which is why it stays,
but the halo-copy and clone alternatives are less dead than the spec said. An
Edit racing a job pays under a microsecond.

`ChunkStore::shared` carries `cfg_attr(not(test), allow(dead_code))` until
ticket 03 calls it; remove the attribute there.
