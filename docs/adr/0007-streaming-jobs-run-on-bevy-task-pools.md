# Streaming jobs run on Bevy's task pools and hold only snapshots

Generation, light solving and meshing run off the thread that draws the frame.
Two decisions about how are cheap now and expensive once other code leans on
them.

**The concurrency lives in `tesserae`, never in `tesserae-core`.** Core's
functions are pure over borrowed data and its types are plain owned data, so
they are already `Send + Sync`; `tesserae` hands them to Bevy's
`AsyncComputeTaskPool` as one job per Chunk. Core keeps zero dependencies - not
`bevy`, not `wgpu`, and not `rayon` - and stays benchmarkable because it has no
opinion about a scheduler. `rayon` is for work that is data-parallel inside one
job, such as a whole LOD level's downsample, where it is called from a job and
not from the tick.

**A job receives shared references and returns an owned value.** The
`ChunkStore` holds `Arc<Chunk>` and `Arc<ChunkLight>`; a job holds clones of the
Arcs it reads and nothing else. There is no `&mut` into World state on a worker.
The result lands on the Server's tick, through the same `pub(in crate::server)`
mutators as any other write, so `docs/adr/0002-client-server-from-day-one.md` is
enforced by the same privacy it always was. Every write goes through
`Arc::make_mut`, which clones only while a job holds the Chunk: a write is never
seen by a job already running, and a held Arc cannot have its address reused, so
`Arc::ptr_eq` against the store says whether a Chunk has changed since the
snapshot.

## Consequences

- **A result is validated where it lands, from state the tick already keeps.**
  Dispatch takes a Chunk out of the stale set, so anything that marks it stale
  while its job runs is still recorded when the job lands. A light solve is
  dropped if an Edit touched anything it read, because the incremental path is
  not monotone and a bulk result computed before it must not land over it, and
  if the Chunk is no longer the same instance. A mesh lands only while its Chunk
  is inside the mesh extent, which also keeps eviction safe without a re-mesh.
- **Light is a relaxation and tolerates stale reads.** Every change a bulk solve
  makes is monotone, so a job that read a neighbour's old plane produces a value
  that is too low, never wrong, and the Chunk is queued again when the plane
  moves. The order jobs land in does not change where it ends.
- **A new stage follows the pattern** or does not run off-thread: a snapshot of
  `Arc`s in, an owned result out, validated and applied on the tick. A stage
  that needs a `&mut` into the store is not a job.
- **The `ShapeTable` is immutable once the World is open**, because jobs hold it
  by `Arc`. If Shapes ever become registrable at runtime, in-flight jobs hold
  the old table and landing needs a table epoch.
- **When work lands is no longer deterministic**, only what is true once it has.
  Tests settle - tick until nothing is in flight - and assert on the quiescent
  state and on bounds that hold whenever they are sampled.
