# 12: Count what a visibility graph would cull or defer

Status: done
Blocked by: 01, 03

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-visibility`.

## What to build

- The 15-bit face-to-face connectivity of each Chunk, as Minecraft and Sodium
  compute it: in the generation job, and again in the mesh job after a change,
  so it stays off the tick (ADR-0007). It ships with the mesh; the Client never
  reads World storage (ADR-0002).
- A `cave` fixture: the Eye inside a cave of the demo's default seed, turning
  through 360 degrees in the measured phase.
- Two profiling-only walks, nothing culled or reordered yet:
  - on the Client, from the camera's Chunk: how many drawn Chunks it would
    cull;
  - on the Server, from the Eye's Chunk: the share of level-0 mesh worker time
    spent on Chunks it cannot reach, such as sealed caves.
- Build Client culling only if it removes at least 20% of drawn Chunks in a
  surface or cave fixture. It must not cull a Chunk 02 is retaining.
- Move unreachable Chunks to the lowest score in 03's order only if they take
  more than 30% of level-0 mesh worker time in a sprint fixture. They are never
  skipped, so entering a cave still finds them meshed.
- Client culling is a per-view optimization, so it is adopted only on 01's GPU
  and render attribution and the spec's effect gate.

## Acceptance criteria

- [x] Connectivity computation costs under 5% of mesh job time.
- [x] Cull fraction and unreachable mesh share recorded per fixture here.
- [ ] If built: zero rule-hole frames, no popping in `cave` review captures,
      and the spec's effect gate passed.

## Comments

### 2026-11-01: counted; culling built behind a switch, off by default

Branch `tech/streaming-visibility`. Client culling's gate waits for the final
review series; Status unchanged.

Built:

- `tesserae_core::visibility::connectivity`: the 15 face-pair bits per Chunk.
  Only a full cube of an `Occlusion::Opaque` Material blocks sight; glass,
  leaves and every other Shape let it through. A flood over one `u32` per row
  of Voxels, read a byte of palette indices at a time
  (`Chunk::rows_where`). Under 1,024 blocking Voxels nothing can separate two
  faces, so the flood is skipped.
- The generation job works it out; the mesh job reuses it unless the Chunk's
  Voxels changed since (the Server keeps a `Weak` to the Voxels it came from,
  which an Edit's `Arc::make_mut` moves). `ChunkMeshReady` carries it; the
  Client never reads World storage.
- Cost, criterion `meshing`, same Chunks: straddling 11.3 against 286 us for
  `mesh_chunk` (3.9%), shore 13.5 against 292 (4.6%), cave crossed 3.4
  against 276 (1.2%), cave mouth 8.7 against 257 (3.4%). The mesh job pays it
  only after an Edit.
- Walks in core: `reach` (where an eye could move through open space) and
  `sight` (Minecraft's: breadth first, entry face to exit face, never back
  against a direction taken).
- `cave` fixture: the Server puts the Eye in a cave within 128 m of spawn,
  under 8 m of ground, on its first tick (default seed: (56.5, 27.5, -100.5));
  `TESSERAE_SCREENSHOT_TURN_DEG=360` turns the camera over 22-32 s. In the
  runner's `MOTION`. The tunnel is narrow: the shutter frame is a near wall.
- Instruments, `--instruments visibility` (`TESSERAE_PROFILE_VISIBILITY`):
  `sight_*` and `mesh_reach_*` columns in `frames.csv`, see frame-profiling.md.
  The Server walk runs on the tick, so it stays out of timing series.
- After 09, a Chunk counts as drawn when its region is in the frustum (its own
  box for a blended layer alone); the count adds quads and whole regions.

Counts: single `shot.sh` runs, release `profiling`, offscreen 1080p, holes
counted, fixture phase. Culled = drawn Chunks the walk does not enter, over
all frames; unreachable = level-0 mesh worker CPU on Chunks `reach` from the
Eye does not enter.

Minecraft's walk, inside the frustum (`bin-12-89dfac1`):

| Fixture       | Chunks | Quads | Regions emptied | Unreachable |
| ------------- | ------ | ----- | --------------- | ----------- |
| stationary    | 40.8%  | 30.2% | 0%              | 0%          |
| sprint-flight | 45.6%  | 40.8% | 25.1%           | 25.7%       |
| sprint-walk   | 36.4%  | 30.2% | 6.5%            | 2.9%        |
| cave          | 53.7%  | 47.3% | 32.1%           | 0%          |

The walk as built, without the frustum, from every Chunk within 4 m of the
camera (`bin-12-5d96f8b`):

| Fixture       | Chunks | Quads | Regions emptied | Unreachable |
| ------------- | ------ | ----- | --------------- | ----------- |
| stationary    | 7.9%   | 2.8%  | 0%              | 0%          |
| sprint-flight | 15.9%  | 8.8%  | 16.7%           | 21.3%       |
| sprint-walk   | 9.7%   | 3.0%  | 3.0%            | 2.3%        |
| cave          | 27.2%  | 19.9% | 17.5%           | 0%          |

About 76-95 level-0 Chunks are drawn a frame. The first table's Client count
read connectivity through a second `ChunkMeshReady` reader, which saw a
message only if it ran before `receive_meshes` drains them (ticket 14); a
missed one reads as open, so it can only have under-counted. Connectivity is
now taken where the messages are drained.

Decisions:

- Client culling is built: the cave fixture removes 27% of drawn Chunks with
  the walk as built, over the 20% bar. `TESSERAE_SIGHT_CULL=1` under `dev` or
  `profiling`, off by default. A Chunk the walk does not enter is left out of
  its region's next assembly, or its blended entity hidden; a retained Chunk
  never is. The walk reruns only when the start Chunks or the connectivity
  change, so turning costs nothing. Without the frustum because Bevy already
  culls by it, and a frustum walk changes on every turn; the 4 m margin covers
  a camera transform a frame behind.
- Unreachable Chunks are not reordered: 21-26% of level-0 mesh CPU in
  `sprint-flight` and 2-3% in `sprint-walk`, under the 30% bar.
- Culled against unculled at the shutter: `cave` byte-identical, `stationary`
  no pixel off by more than 24 levels. Zero rule-hole and fill-hole frames in
  every run, culled or not. Single runs: frame p50 3.53 culled against
  3.70 ms in `cave`, 5.19 against 4.93 in `stationary`; not evidence.
- Known limits: a culled Chunk casts no shadow either, as Bevy's shadow pass
  draws the same region mesh, so a hill hidden behind a ridge could lose its
  shadow on ground in view. The walk misses a line of sight that bends back,
  as Minecraft's does.

Pending the final series: culling on against off in `cave` and a surface
fixture (frame p50/p99, GPU attribution, regions rebuilt), and a popping
review of the turning `cave` capture. For your decision: the frustum walk culls
three to five times more at the surface (first table), at a walk and region
rebuilds on every frame of a turn.

### 2026-11-01: culled Chunks keep their shadows

You chose to keep shadows. Effect gate still pending the final series.

- How: a region's culled members go into a second mesh on render layer 32
  (`SHADOW_ONLY_LAYER`, past every level's), which the sun's layers include
  and the camera's do not. Shadow views draw every member; the camera's main
  and prepass views draw only what the walk reaches.
- Cost: shadow geometry is what it was without culling, plus one draw per
  cascade for each region with a culled member. A change in the culled set
  rebuilds two meshes of a region instead of one. Nothing is stored twice.
- Not taken: keeping culled members in the region and discarding them by a
  per-Chunk bit in the main and prepass shaders. Their vertices would still be
  shaded and rasterised there, which is the work culling exists to remove.
- Test: `a_chunk_culled_from_the_camera_still_casts_its_shadow`. A Chunk
  behind a sealed one leaves the camera's region mesh and lands, whole, in a
  caster on a layer the sun draws and no level's camera layer does, without
  `NotShadowCaster`; when the ridge opens, it moves back.
- `hidden-hill` fixture: in the sky over spawn, a ridge (a full plane of the
  Chunk east of the Eye's), a hill behind it and a floor under the hill's
  Chunk, seen under the ridge's lower edge. The walk culls the hill's Chunk,
  and its shadow falls on the floor in view
  (`the_hidden_hill_is_culled_and_shadows_floor_the_eye_sees` checks both
  from the plan). `TESSERAE_SIGHT_CULL=noshadow` culls without the shadow
  meshes, as a control.
- Captures (`bin-12-e149d58`, `shot.sh`, culling off against on):
  - `hidden-hill`: 0 pixels differ. The control loses the hill's shadow on
    the floor: 3,546 pixels differ by more than 24 levels.
  - `cave`: 0 pixels differ.
  - `stationary`: 1,622 pixels differ by at most 14 levels, at a cave mouth
    and the horizon. Two unculled runs differ by up to 54 there, with 16 in
    the cave mouth, so this is run-to-run, not shadow.
- The walk culls 3.7% of `hidden-hill`'s 27 drawn Chunks and one region of
  eleven: the hill's.

### 2026-11-01: final series; culling not adopted

Final series `/tmp/claude-streaming-impl/final`: release `profiling` builds,
offscreen, two interleaved rounds (cut from three), one binary per merge in
order (`bNN` is the integration branch just after ticket NN), the tip `075ff52`,
and the tip with one switch changed. Medians of the two runs; a range is the two
runs, low-high.

- `TESSERAE_SIGHT_CULL=1` at the tip: `cave` frame p50 3.62 ms against 3.60,
  drawn meshes 109 against 114; `stationary` 4.57 against 4.54. No effect, so
  culling is not adopted.
- The client cull path is removed. Connectivity and its counts stay as a `dev`
  and `profiling` instrument; release builds do no connectivity work.
