# Streaming, LOD and rendering performance

Status: shared understanding confirmed 2026-10-30; ticket 01 authorized for
local implementation. Push, PR and later tickets each need your approval.

Source baseline: `main` at `446e984`. Decisions and their history:
[grilling.md](grilling.md). Primary-source findings: [research.md](research.md).
Tickets: [issues/](issues/).

## Problem

Testing PR 7 (`feat/world-seed-selection`, since merged as #30), whose demo
terrain has 24 m relief, caves and lakes, you reported:

- frame rate well below Minecraft with Distant Horizons or Voxy, and Luanti,
  and Chunks loading slower than in those;
- empty patches in the ground when flying up, down or towards the ground,
  filling in later;
- sprinting dropping to about 30 fps, recovering when you stop.

One unmatched series each (release, profiling, 1080p offscreen, host load
about 6): the flat demo against the new one moved stationary frame p50/p95
from 6.15/8.34 ms to 8.42/11.42 ms and flight from 6.16/7.89 to 8.47/11.65.
Server p95 went from 0.3 to 2.4 ms, from lake and scenario water. Main p50 went
from about 1.7 to 3.2 ms and render CPU p50 from 2.9 to 3.6 ms. Frame p50 sits
well above both, so the GPU or waits are unattributed. These numbers are
context, to be re-measured by ticket 01.

## What the code shows

Figures are the engine defaults (192 m view, 4096 m LOD reach); the demo sets
160 m and 3000 m, which the fixtures run.

- **Holes.** `chunk_material.wesl` discards every coarse fragment within a
  metre radius and half height of the Eye (144 m and 94 m at level 1 by
  default, about 113 m in the demo), whether or not level 0 has delivered
  there. A coarse camera's near plane is a quarter of the view distance (48 m,
  40 m in the demo). The Server unloads a coarse Chunk once it is a hysteresis
  Cell inside the finer level's hole, whether or not the finer Chunks exist.
  The reverse happens flying up: level 0 unloads before level 1 has meshed.
- **`drawn_mask`** only stops coarse *water* from drawing twice; opaque and
  cutout get an empty buffer. Its delivered set includes empty meshes, which
  spawn no entity.
- **Order.** Level-0 generation and meshing are nearest-first, relight
  highest-first. Coarse dispatch walks the levels in order over nearest-first
  offsets, and stops when the budget runs out, so level 1 runs before level 2
  whatever is on screen; landing sorts by `(level, distance)`. The pool is
  FIFO; only what is dispatched can be ordered.
- **Coarse generation.** A coarse Chunk generates its eight children (about
  250 us each) and downsamples them. Every level generates its whole load
  extent, including the region its finer level draws, which may be prefetch
  on purpose. Level 1 smooths slopes; levels 2 and up do not.
- **Rendering.** A 40 B vertex, one entity per Chunk per layer, and six
  cameras, one per LOD level, each clearing depth, so finer levels paint over
  coarser ones and every covered coarse pixel is drawn and overdrawn. Mesh
  attributes are cloned on the main thread.
- **Workers.** Bevy's default split, three async compute threads on this M5,
  shared by streaming, LOD, fluids and Mods; `TESSERAE_ASYNC_THREADS` overrides
  it.
- **Sprint.** Flight sprint is capped at 64 m/s (ADR-0008), crossing a 32 m Cell
  every half second; physics sprint is 8 m/s. Which one you saw stall is
  unknown, so both are measured. Your main display runs at 144 Hz, so about 30
  fps is a 28 to 35 ms frame, not vsync halving.

## Decisions

- **No gaps.** A level is drawn until delivered coverage from another level
  replaces it, in both directions. A level-k fragment is discarded wherever any
  finer level has delivered the Chunk at that point, an empty mesh included;
  delivered means membership of the Client's existing delivered set. All three
  materials read it. Retention is Client presentation only (ADR-0002): a
  retained mesh is released once its region is covered (a coarse Chunk by all
  eight of its children, recursively; a finer one by its coarser parent), or
  once it is outside every level's mesh shell. The metre cut and the coarse
  near plane go. A brief blocky patch is accepted in place of a hole. Recorded
  as a new ADR.
- **Two kinds of hole.** A *rule hole* is a region some level has delivered or
  retains that nothing draws; a *fill hole* is a region no level has delivered.
  The draw rule fixes rule holes; order and throughput fix fill holes.
- **Priority on the Server.** One score from the Server's own Player state: the
  Eye predicted 1 s ahead, a fixed 60 degree half-angle cone with its apex 6 m
  behind the Eye (the Server knows no Client field of view, and the
  third-person camera sits up to 6 m back), a 2-Cell turn margin and an age
  credit. Holes first, coarsest missing level first, then refinements. Relight
  by column score, top-down within a column. "Hole" on the Server means
  nothing sent and still held at any level, since it cannot see what the
  Client retains.
- **Hidden work is reordered, never skipped.** The Server needs every Chunk in
  the load ring for collision, simulation, light and Mods. The existing skips
  for uniform air and buried uniform solid stay. Visibility culling is a
  render-side ticket, counted before it is built; meshing Chunks a walk from
  the Eye cannot reach may drop to the lowest priority behind a measured gate.
- **Coarse generation.** Profile first; any cut keeps output bit-equal. Cheaper
  far levels that change what is drawn are probes you approve on captures.
- **Coarse persistence** is a discardable cache behind a measured gate, never
  authoritative. ADR-0011 is amended only if the gate passes.
- **Rendering stays inside bevy_pbr and `MaterialExtension`.** Collapse the
  cameras once coverage makes one depth buffer correct, then an exact 16 B
  packed vertex (per-quad packing only if that is not enough), then opaque and
  cutout batching. Bevy's occlusion culling is an experiment. All of it is
  blocked on GPU attribution, as frame-profiling.md requires. Moving the mesh
  attribute clone off the main thread is separate and needs only ticket 01.
- **Fluid is out of scope** and gets its own spec. Its share is reported
  separately so frame numbers are not charged with it.

## Targets

Matched interleaved release series on this Mac, under frame-profiling.md, at
unchanged view distance and visual quality, in each fixture's measured phase
(22 to 32 s):

- stationary and flight: frame p95 at most 8.3 ms;
- both sprints: frame p95 at most 16.7 ms and no frame over 33 ms;
- every fixture: zero rule-hole frames;
- dive and both sprints: zero fill-hole frames;
- stretch, not a gate: stationary at 6.9 ms (144 Hz);
- time from a teleport to `streaming_settled`: reported, no fixed target.

The **effect gate** for every ticket: a median improvement of at least 5% that
also exceeds the profiler's logging overhead bound and the between-run
variation (run ranges do not overlap), with no regression in p99, streaming
readiness or simulation progress. Core algorithm changes use the core
comparison methodology in frame-profiling.md instead. Ticket 15 checks the
targets themselves.

## Tickets

| #  | Ticket                                       | Blocked by     |
| -- | -------------------------------------------- | -------------- |
| 01 | Streaming fixtures, attribution and baseline | none           |
| 02 | Draw a level until its replacement lands     | 01             |
| 03 | Dispatch by view and direction of travel     | 01             |
| 04 | Profile and cut coarse generation            | 01             |
| 05 | Decide whether to cache coarse levels        | 04             |
| 06 | Quality probes for cheaper far levels        | 04             |
| 07 | One depth buffer instead of six cameras      | 01, 02         |
| 08 | Pack the Chunk vertex                        | 01             |
| 09 | Batch opaque and cutout                      | 08             |
| 10 | Experiment with Bevy's occlusion culling     | 07             |
| 11 | Tune the workers                             | 03, 04         |
| 12 | Count what visibility would cull or defer    | 01, 03         |
| 13 | Cut level-0 job cost                         | 01             |
| 14 | Upload meshes off the main thread            | 01             |
| 15 | Check the targets                            | 02 to 14       |

Ticket 01 ends with an attribution checkpoint. Tickets 02 to 14 may then be
reordered, not re-scoped, with your approval.

## Prior art

Read from source at pinned commits; citations in [research.md](research.md).

- **No holes.** Voxy (32-cubed sections at every level, as here) keeps the
  invariant "leaf nodes always contain geometry": a parent draws, and asks for
  its children, until every child mesh is resident. Distant Horizons disables a
  parent only once all four children can render. Luanti and Sodium keep a mesh
  until its replacement is applied. Ours is Voxy's invariant applied per Chunk:
  a delivered child replaces its part of the parent at once, instead of all
  children together.
- **Order.** Sodium scores a task as build type + distance terms - 1 per 5 s
  waited - 3 if in the frustum: off-screen work is deferred, never starved.
  Luanti pads the send cone to `cos(fov * 0.55)`, a "+10%" hotfix, narrows it
  with speed and adds a 5 degree cone along the velocity, and restarts its scan
  on a turn of 10% of the FOV. Distant Horizons picks the minimum of
  `chebyshevDist / detailLevel`, "generate low detail LODs first": a continuous
  form of our holes-first pass, and a candidate for ticket 03.
- **Visibility.** Minecraft stores 15 face-pair bits per section and walks a
  BFS from the camera's section. Sodium uses up to four directional sets per
  section, angle masks and an async BFS, and builds mesh tasks only from what
  the BFS visits, so occluded sections are never meshed. Luanti culls on the
  Server before sending ("50-80%" fewer blocks) and on the Client with a BFS
  over six solid-face bits. Vintage Story ray-tests (2018 blog; closed source).
- **Far data.** Distant Horizons stores 64x64 columns of 64-bit run entries in
  SQLite keyed by `(DetailLevel, X, Z)`, skips unchanged chunks by hash, and has
  generation modes from surface-only (no caves) to full. Voxy stores palettized
  32-cubed sections in RocksDB and builds only from chunks the client received;
  it has no generator. Both persist because their source data is expensive or
  absent; ours is a generator.
- **Vertices.** Sodium 20 B per vertex, Distant Horizons 16 B per vertex, Voxy
  8 B per quad (vertex pulling, 5-bit position, 4-bit size). Ours is 40 B.
- **Metal.** wgpu 30.0.1, which Bevy 0.20 uses, has no
  `multi_draw_indirect_count` on Metal (an empty TODO) and emulates
  `multi_draw_indirect` as a CPU loop; `INDIRECT_FIRST_INSTANCE` and binding
  arrays are supported. Voxy's GPU-generated draw count does not port. By
  source, Bevy 0.20 enables GPU culling on Metal (unconfirmed at runtime).
  `OcclusionCulling` is opt-in two-phase culling needing `DepthPrepass`, with
  known false occlusion. A non-bindless `MaterialExtension` batches only with
  meshes sharing its material instance and slab, which bounds ticket 09.

## Constraints and seams

- `tesserae-core` stays free of Bevy, wgpu and rayon; no Client system reads
  World storage (ADR-0002); everything physical in metres (ADR-0003); jobs
  follow ADR-0007.
- Measurements follow frame-profiling.md: interleaved rounds, executable
  hashes, no competing builds, and wait for `/tmp/tesserae-frame-profile.lock`.
  Captures stay in one `/tmp` folder and are deleted after review.
- PR 7 merged as #30, so the baseline uses `main`'s own demo dist, pinned by
  hash. The old flat demo is context only.
- Ticket 07 meets `client/camera.rs` as #29 (`0c913c3`) left it.
- This branch ships the spec, the tickets and ticket 01. Every later ticket is
  its own `tech/streaming-<slug>` branch and PR.
