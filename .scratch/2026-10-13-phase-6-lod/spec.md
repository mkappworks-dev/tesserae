# LOD mip chain out to 4 km

Build-order step 6. Branch `feat/phase-6-lod-mip-chain`.

Every Chunk is the same 32^3 at every LOD level and a Cell is its footprint in
metres, so a Chunk of level N+1 covers 2x2x2 Chunks of level N. This step writes
the downsample that turns eight finer Chunks into one coarser one, streams a
ring of Chunks per level around the Player's Eye out to a configured reach (4 km
by default), meshes them with the mesher that already exists, and draws them
without cracks or flicker where one level meets the next.

Two things were decided before this spec and are followed, not argued again.
Both are in `.scratch/2026-10-12-phase-5-shaped-voxels/spec.md` decision 10 and
in `ADR-0010`:

- A Shape becomes a full cube or air by **volume**. A coarse Voxel is solid if
  its eight children's `ShapeTable::volume` sum to at least half of 8 x 4096,
  and ties go to solid. So no level above 0 ever holds a shaped Voxel and pass 2
  of the mesher never runs there.
- A connected Shape (a fence) is volume-only at coarse levels, which is the bare
  post's volume. A connection is derived from neighbours and never stored, so
  there is nothing to downsample.

## Vocabulary this step adds

Two terms, to be added to `CONTEXT.md` in the same PR.

- **LOD ring** - what one LOD level draws: the Cells between an inner and an
  outer distance from the Eye, in metres. The outer distance doubles with each
  level.
- **Downsample** - producing a coarse Chunk's Voxels from the eight finer Chunks
  it covers: the dominant Material by volume, solid if half the volume is.

## What the design rests on

Read from the code before deciding, because three of the decisions below only
make sense once these are known.

- **A coarse Chunk cannot be mipped from stored finer Chunks.** The Server holds
  Level 0 only within `evict` (288 m at the defaults) of the Eye, so the Chunks
  a level-1 Chunk at 300 m would be made from do not exist. A level N+1 Chunk is
  therefore downsampled from eight level-N Chunks that are **generated for the
  purpose**, not stored: `Worldgen` is a pure function of seed, Voxel size and
  parameters, and everything in it is tuned in metres, so
  `Worldgen::new(seed, voxel_size * 2^N, params)` is the level-N generator and
  needs no change. That is eight generator calls per coarse Chunk, not 8^N, and
  the generator's early-outs (4 ns for air and deep rock) make most of them
  free.
- **The mesher already works at every level.** `mesh_chunk` emits positions in
  Voxel-index space and `chunk_mesh_to_mesh` is "the one place Voxel-index space
  becomes metres". A level-N mesh is scaled by `voxel_size * 2^N` instead of
  `voxel_size` and nothing in `tesserae-core` changes. Mesh quads, merging,
  occlusion and the Material table behave the same.
- **Bevy 0.20's projection is infinite reverse-Z with an f32 depth buffer**
  (`PerspectiveProjection` builds `perspective_infinite_reverse`; `far` only
  places the frustum's culling plane). Depth precision at 4 km is not the
  problem DESIGN.md's "separate target with its own depth" technique was
  invented for in a GL-class renderer. What it still buys is a clean rule where
  two levels overlap: see decision 8.

## Decisions

### 1. Coarse levels are derived data, never stored, never edited

A level >= 1 Chunk is a pure function of the seed, the parameters, and the
level-0 Chunks the Server currently holds that an Edit has changed. It is not
save data and no Edit is ever applied to it. The `ChunkStore` and everything the
edit path touches (light, relight, `MeshState`, `set_voxel`) stay level 0 only.

This keeps ADR-0002 intact by construction: the Server remains the only writer,
and the new store is private to `crate::server` for the same reason `ChunkStore`
is. It also means the save format step 6 was supposed to have settled is
unchanged: the format has been LOD-aware since step 1 (Chunk is 32^3 at every
level, Cell is its footprint) and a later save step stores level 0 only and
rebuilds the rest, as this step does.

What is lost, declared here: an Edit is visible at level 0 and level 1 and at no
level above, and an Edit survives only while its level-0 Chunk is held, which is
already true today because there is no save. Both stop being true when a save
step makes level 0 durable; the coarse levels then read it instead of the
generator and need no other change.

### 2. The downsample

```rust
pub fn downsample(children: [&Chunk; 8], shapes: &ShapeTable) -> Chunk;
```

in `tesserae-core/src/lod.rs`. Child `i` is the octant `i = x + 2y + 4z` with
`x`, `y`, `z` in `{0, 1}`; the coarse Voxel at `(cx, cy, cz)` covers the 2x2x2
Voxels at `(2cx + dx, 2cy + dy, 2cz + dz)` of the 64^3 lattice the eight
children form. No Material table is needed: dominance is by volume and
translucency is not consulted.

**Weight.** A child Voxel weighs `shapes.volume(v)`, out of 4096: air 0, a cube
4096, a slab 2048, a stair 3072, a fence post its bare variant's volume. Air is
0 whatever shape byte it carries.

**Solid or air.** Let `S` be the eight weights' sum. The coarse Voxel is solid
if `2 * S >= 8 * 4096`, else `Voxel::AIR`. Ties are solid, which is the decision
already made: a slab floor that vanished at distance would be a far worse
artefact than one that thickened. A solid result is always a full cube
(`Voxel::cube(material)`), so a level >= 1 Chunk never holds a shaped Voxel.

**Which Material.** Sum the weights per Material over the eight children and
take the largest total. Ties are broken by where the Material sits, and then by
id, because the first tie that matters is the surface itself:

1. the tied Material whose highest child (largest `dy`) is higher wins, because
   a coarse Voxel is seen from above and a grass cap one Voxel thick over dirt
   is exactly a four-to-four tie at every odd surface height;
2. then the lower `MaterialId`, so the answer never depends on palette order or
   iteration order.

Without rule 1, `DIRT < GRASS` would turn every distant hill brown, and a test
pins it.

**What a translucent Material becomes.** Nothing special, deliberately. Glass,
leaves and water keep their own `MaterialId`; they weigh like any other cube, so
they win a Voxel only if they hold the most volume, and they count as occupied
for the solid test. What they do after that is the Material table's business,
exactly as at level 0: their render class sends their faces to the blended
layer, and their occlusion decides what they hide. The mesher is not told the
level. Two consequences, both accepted: a pane one Voxel thick disappears at the
next level (it holds under half a coarse Voxel), which is what a mip does to
anything thinner than its texel; and leaves, whose occlusion is `Never`, draw
every face between two coarse leaf cubes as they do at level 0. Fewer, larger
Voxels make that cheaper per metre, not dearer. The client has no blended draw
call yet (ticket 04 of `2026-10-12-material-table-transparency`), so a coarse
water or glass layer is built and sent and not drawn, as at level 0.

**Emitters.** Not preserved. A coarse Voxel is its dominant Material, which is
almost never a lantern, and a coarse level carries no block light at all
(decision 3), so an emissive Material has nothing to say at distance. Nothing
generates one today (the lantern is placed by a player or the `dev` gallery, and
neither can reach a level >= 1 Chunk by decision 1). When a save step makes
placed lanterns durable, "a coarse Voxel holding an emitter keeps its Material
if the half-volume test passes" is the one-line change, and it is listed under
Known loose ends.

**Fast paths.** Eight identical uniform children whose weight passes the test
return `Chunk::filled` without a loop, and eight air children return air. A
mixed Chunk takes the general loop. The general loop is measured before it is
optimised, and any tightening is benchmarked against the plain version first, as
step 4 learned.

**Where it runs.** In a Job, on the async compute pool, as one stage with the
generation of its eight children (decision 5). `rayon` is not used: the unit of
parallelism is a Chunk and the pool already runs many at once, and ADR-0007
keeps it out of core. `tesserae-core` gains one module and no dependency.

### 3. Light at coarse levels

A level >= 1 Chunk carries **no stored light**. `ChunkLight` is not allocated
for it, no relight stage runs for it, and the flood in `light.rs` is untouched.
The mesher is handed a `ChunkLight::filled(Light::new(MAX_LIGHT, 0))` for the
centre and for each of its six neighbours, so the vertex light attribute on
every coarse face is full sky and no block light.

Why that is the right amount:

- Open terrain at level 0 is uniformly sky 15, so a coarse face matches the
  level-0 face next to it, and the shader's `diffuse_occlusion` and
  `directional_shadow_factor` take the same inputs on both sides of a ring.
- Light is part of the greedy merge key. A uniform value merges maximally, so a
  coarse mesh has fewer quads than the same terrain lit, which is the cheaper
  direction for the 4 km target.
- Block light is a local source and sub-pixel at the distance a coarse level
  starts at. Sky light under an overhang or inside a cave mouth is the one thing
  lost: a distant tunnel's far wall is lit like open sky. At level 1 (2 m
  Voxels) that is a hole in a hillside with a bright interior, at 400 m. It is
  declared under Known loose ends with the cheap fix, a column rule computed in
  the generation job, which needs no change to the flood.

The light solver's cost therefore does not grow with the number of levels, and
`light.rs` takes no new rule. That matters more than it looks: its speed depends
on const generics, and a rule added there unconditionally has cost before.

### 4. Rings: levels, shells and the hole

Everything is derived from metres, so a World with Voxels of 0.25 m gets the
same rings in the same distances and a quarter of the Chunks per metre
(ADR-0003, "View distance in metres scales as 1/`voxel_size`").

`StreamingSettings` gains `lod_distance_m` (default 4096): the reach of the
outermost ring. `view_distance_m` (default 192) stays the level-0 radius, so it
is the first ring's outer distance.

- **Level count.** The smallest `n` with `view_distance_m * 2^(n-1) >= reach`:
  `1 + ceil(log2(reach / view_distance_m))`. At the defaults that is 6 (levels 0
  to 5), not the 8 DESIGN.md's "~8 levels" assumed. It assumed a smaller
  full-detail radius; 8 is what a 32 m radius gives, and that is the knob.
  `lod_distance_m <= view_distance_m` is a World with no coarse levels, and the
  Server then does exactly what it did before this step.
- **Level k, k >= 1.** Voxel `voxel_size * 2^k`, Cell `cell_size * 2^k`. Its
  outer extent is `ceil(min(view_distance_m * 2^k, reach) / cell_k)` Cells,
  which is 6 for levels 1 to 4 and 4 for level 5, and its vertical extent is the
  same Cell count as level 0's (3), so each level holds about as many Chunks as
  level 0 and the total grows with the number of levels and not with r^2. This
  is what DESIGN.md's "each level doubling its span" means once written down:
  the ring is a constant number of its own Cells thick, so the screen-space size
  of a Voxel is constant from one ring to the next.
- **The hole.** Level k does not mesh where level k-1 does. It is a cylinder
  of radius `H` and half-height `V` of level-k Cells around the anchor, derived
  and not rounded, because both levels floor the same Eye to their own lattice:
  the finer anchor is the coarse anchor's first or second child on each axis,
  so the finer disc is off-centre by up to a finer Cell, and an overlap
  measured from the anchor's centre is a Cell on one side of the Eye and none on
  the other. The rule is checked for every one of the four parities.
  - _Horizontal._ The largest `H` such that the hole's Cells, grown by one Cell
    of level k all round, lie inside the finer level's disc of `R` finer Cells.
    A Cell at offset `i` reaches `2i + 1` finer Cells out, the overlap adds 2,
    and the worst parity puts both axes on their far side, so
    `(2i + 3)^2 + (2j + 3)^2 <= R^2` for every `(i, j)` on the hole's rim. At
    the defaults `R` is 6 and `H` is 1; the first arithmetic, `(R - 2) / 2`,
    gave 2, which has no overlap at all for half of the Eye positions.
  - _Vertical._ `(Rv - 1) / 2` floored, so the slab lies inside the finer rows
    for either parity. At the defaults `Rv` is 3 and `V` is 1; it was 0, which
    left every level drawing the terrain above and below the Eye in the same
    column as the next level. There is no vertical overlap: a column is drawn by
    one level where the hole is, and the coarse level takes it over where the
    finer level's rows end.
  - Where no radius fits (`R < 5`) there is no hole.

  At the defaults that is a hole of radius 1 and half-height 1 against a finer
  extent of radius 6 and half-height 3. The overlap is the seam strategy of
  decision 9: it is why a crack between two levels has something behind it.
- **Generation has no hole.** `load` is the outer extent grown by one Cell (so
  every meshed Chunk has its six neighbours, as at level 0) and `evict` is
  `load` grown by the hysteresis, for every level. The hole removes a few
  hundred uniform Chunks from generation and a few draw calls from the Client; a
  hole in `load` would cost a second kind of extent for nothing.
- **Hysteresis.** `hysteresis_m` is scaled per level like the distances, which
  is the same number of Cells (1) at every level, so a level changes state when
  the Eye has moved one of _its own_ Cells. A Chunk keeps its mesh while it is
  inside `outer.grown(hysteresis)` and outside the hole **shrunk by the
  hysteresis in the horizontal only** (radius 0, half-height 1 at the
  defaults), so the coarse layer under the fine one is released when the Eye
  walks onto it and not a Cell later.
- **Anchor.** Level k's anchor Chunk is `floor(eye / cell_k)`, from the same Eye
  the Server already streams around.

The shell type is `Extent` plus an optional hole; `Rings::new` becomes
`Rings::for_level`. Level 0 is built through the same constructor and must come
out equal to today's `Rings` (a test), which is how "this step changed nothing
for level 0" is checked.

### 5. Streaming coarse levels, on ADR-0007

A coarse level follows ADR-0007 to the letter: a Job receives `Arc` snapshots
and returns an owned value, the result is validated where it lands from state
the tick already keeps, and nothing on a worker holds a `&mut` into World state.

**Two stages, not three.** There is no relight, because there is no light.

1. _Generate_ a coarse Chunk. The job closes over a clone of the level below's
   generator and, for level 1 only, up to eight `Arc<Chunk>` taken from the
   `ChunkStore` at dispatch for the children it already holds. A held child is
   used instead of generated, which saves the work and is the only way an Edit
   reaches a coarse level (decision 6). The job generates the rest, downsamples,
   and returns the `Chunk`.
2. _Mesh_ it. The job holds `Arc<Chunk>` for the Chunk and its six neighbours of
   the same level, the synthetic light of decision 3 and the `Tables`, and
   returns a `ChunkMesh`. It is `build_mesh` with a different light.

**The source generators.** `Worldgen` for source level j is built once, at world
open, with `voxel_size * 2^j`. Level 0 is the production generator, caves and
smoothing included, so level 1 downsamples exactly what level 0 holds, stairs
and all, and the volume rule is exercised where it can matter. \*\*Sources for j

> = 1 are built with `smoothing: None`.\*\* `Worldgen::new` gates smoothing on a
> one-Voxel rise being at least `min_rise_m`, so a 2 m Voxel passes it and would
> place 1 m stairs that the next downsample then rounds back to cubes: wasted
> work, and the gate is by metres so it does not turn itself off. Caves stay on,
> being sized in metres: the same Cave at any Voxel size, aliased to whatever
> the Voxel can show.

**State.** A Server-private `LodStore` holds, per level,
`HashMap<ChunkCoord, Arc<Chunk>>`, and a `LodState` holds per level what
`MeshState` holds for level 0, minus light: `dirty` (needs a mesh), `stale`
(needs regenerating), `delivered`. `Stage<T>` is keyed by `ChunkCoord` today; it
gains a key parameter defaulting to it, and the coarse stages are keyed by
`(level, coord)`, so the level-0 stages and their tests are untouched.

**Validation at landing**, from state the tick already keeps:

- _Generate_ lands if the coord is inside `evict` of the current anchor of its
  level and either absent or marked `stale`. Otherwise it is dropped. Dispatch
  removes the coord from `stale`, so a mark made during the flight is still
  there when the job lands and a fresh job follows; the landed Chunk is no
  staler than the one it replaces.
- _Mesh_ lands if the Chunk is inside its level's `outer.grown(hysteresis)`. If
  it was marked `dirty` during the flight it is delivered anyway and stays
  dirty, for the starvation reason ADR-0007 gives.
- The eviction argument carries over per level: `evict` is wider than `load`,
  which is wider than `keep`, so an evicted Chunk is never adjacent to one the
  Client holds a mesh for.

**Replacement dirties neighbours.** A level-0 Chunk arriving cannot change a
mesh the Client holds, because meshes wait for six neighbours. A coarse Chunk
_regenerated_ in place can, so landing a regeneration marks the held face
neighbours dirty. That is the only case; the first generation of a coord does
not.

**Dispatch order and caps.** Coarse levels run after level 0 each tick, nearest
level first and nearest Chunk first within a level. The caps are _shared across
levels_ and separate from level 0's, so a burst of coarse work cannot occupy the
slots level 0 needs:

```rust
pub struct LodBudget {
    pub in_flight: LodCounts,       // generate, mesh
    pub landed_per_tick: LodCounts,
}
```

The starting values are 16 / 32 in flight and 16 / 16 landed, **guesses that
ticket 07 replaces with measurements**. Their cost model is ADR-0007's: a bad
number is fill latency, never frame time. What is new is that the pool is one
FIFO, so coarse jobs queue ahead of a later level-0 job; ticket 07 measures
level 0's fill with and without coarse levels running, and a second pool for
coarse work is the stated fix if it shows.

**No per-tick scan that scales with levels.** Level 0 scans its offset lists
every tick (about 2000 lookups a stage). Six levels of that would double the
tick for no work, so a coarse level is scanned only after something that can
create work happened: its anchor Chunk changed (every `32 * 2^k` m of travel), a
job of its landed, or an invalidation. `retire` is gated the same way. The
offset lists are shared between levels with the same extent, which is five of
six.

### 6. Edits and coarse levels

An Edit changes one level-0 Voxel in one level-0 Chunk. The Chunk that covers it
at level k is `floor(coord / 2^k)` per axis. By decision 1 only level 1 reads
held level-0 data, so an Edit invalidates **one** coarse Chunk: its level-1
cover. `MeshState::edited(coord)` already is the single place every Edit passes
through, so it gains one call into `LodState`: if the cover is held at level 1,
mark it `stale`; if not, do nothing, because it will read the edited Chunk the
next time it is generated.

The mirror image: when a level-0 Chunk **leaves the store while it has been
edited**, its level-1 cover is marked `stale`, because the Edit is about to stop
existing and the coarse Chunk would otherwise draw a ghost of it. Unedited
Chunks leave without any mark: the generator reproduces them exactly, which is
the property decision 1 stands on and a test pins. The set of edited Chunks is
small and lives in `MeshState`.

Levels 2 and above are generated from the generator alone, so they never need
invalidating. An Edit is at most a few metres from the Eye (`EDIT_REACH_M`), a
level-2 ring starts at 256 m, and a level-0 Chunk is evicted at 288 m: the two
regions cannot be the same one.

The Client is never involved. The coarse Chunk is regenerated by a Job on the
Server's own tick like any other; ADR-0002's "the Server is the sole authority"
is one resource and one hook inside `crate::server`.

### 7. Meshes and the Client's view of them

Two new messages in `server/streaming.rs`, beside the level-0 pair:
`LodMeshReady { level, coord, mesh }` and `LodUnloaded { level, coord }`,
written only by the tick like the others. Level 0 keeps its messages and its
code; a generic "level" field on them would put a field on every level-0 test
for the benefit of a path that does not use it.

The Client's entity map becomes keyed by `(level, coord)` with level 0 as the
existing case. A level-k entity is placed at `coord * cell_size * 2^k` and its
mesh is uploaded with `voxel_size * 2^k`, so `chunk_mesh_to_mesh` takes the
scale it already takes. The entity carries `MeshTag(level)`, which the shader
reads for the debug tint (decision 8), and the render layer of its level's
camera.

### 8. Rendering: a camera per level, composited

DESIGN.md says to render LOD levels into a separate target with their own depth
and composite. Read from the 0.20 sources, this is nearly free and needs no
extra texture, and what it is for is different from what the sentence implies.

**What Bevy does.** Cameras that share a `RenderTarget` share its main colour
textures (`prepare_view_targets` keys them by target, format, usage and MSAA)
and its depth texture (`prepare_core_3d_depth_textures` keys by target and
MSAA). Every `Camera3d` clears depth at the start of its own main pass unless
its `depth_load_op` is `Load`, and a camera with `ClearColorConfig::None` and a
higher `order` draws over what is already there, with MSAA written back
(`MsaaWriteback::Auto`). So a second camera **is** "its own depth, then
composite": the later camera draws on top of the earlier one wherever it has
geometry, and wherever it does not the earlier one shows through.

**What it is for here.** Not precision: Bevy's projection is infinite
reverse-Z. The rule it gives is _coverage priority_: a pixel a finer level's
camera drew shows that level whatever the coarser geometry's depth is. A level
is a cylinder around the Eye and the cylinders are nested, so along any ray from
the Eye the finer level's geometry comes first and a ray leaves a cylinder once
and never returns; coverage priority and depth order therefore agree everywhere
except in the overlap of decision 4, where the coarse level was drawn _under_
the fine one on purpose. There the priority is what stops the coarse level
poking through.

**What the first build did and why it changed.** The spec put every coarse
level in one far camera and one depth, and tied a coarse vertex's depth to its
level with a push along the view ray. Captures showed that cannot work: a coarse
surface up to one coarse Voxel above the finer one is `dh / sin(angle)` further
from it along the ray, which at any grazing view is far more than a push of a
fraction of a Voxel, so coarse blocks stood out of the finer terrain across the
whole overlap. One camera per level makes the finer level win by construction,
and the push is gone.

**The layout.**

- A near camera, `clear_color: None`, the highest `order`, `RenderLayers` {0}:
  level 0, the UI and everything that is not terrain. It is the camera the rest
  of the Client finds (`MainCamera`).
- One camera per coarse level `k >= 1`, `RenderLayers` {k}, `order` falling as
  `k` rises, so the coarsest runs first and is the only one that clears, and
  each finer level draws over the last. Each is a child of the near camera with
  an identity transform, so it cannot drift, with a perspective projection whose
  `far` is `lod_distance_m` plus a Cell (the frustum's culling plane, the one
  thing `far` does here) and whose `near` is a quarter of `view_distance_m`.
  That near plane clips coarse geometry closer than 48 m, which a finer level
  always covers, so a coarse peak cannot fill the view of an Eye hovering beside
  it. A World with no coarse level still has one such camera, to clear.
- A level `k` entity is on layer `k`.
- Two suns with the same transform. A light's layers are both the views it
  lights and the mask of shadow casters for every one of those views, so one sun
  on every layer would put level 0 into every coarse camera's cascades too. The
  sun on {0} casts shadows; the sun on {1 ..} does not, and coarse terrain
  receives none.

**What it costs.** Each camera is a pass of its own: a depth clear, an MSAA
writeback and a resolve. On the M5 at 1080p, offscreen, the median flight frame
is 4.0 ms with the two-camera layout the first build had and 5.6 ms with five
coarse cameras (four more), so about 0.4 ms a camera (Outcome). The two-camera
layout is the cheaper one and is wrong, for the reason above.

**A debug tint.** The chunk material gains a uniform that tints each Material by
its level, set from `TESSERAE_LOD_TINT` under `dev`, so a screenshot can show
the rings as rings. It is a uniform, not a shader def, so it does not specialise
a pipeline.

### 9. Seams

A seam is where a level-k face meets a level-(k+1) face: the finer surface is
never at the coarser one's height (a coarse Voxel is the dominant Material of
eight, so its top is within one coarse Voxel of the fine surface), and a face of
one is culled against a neighbour the other level does not draw.

- **Within a level**, nothing is new: a level's Chunks mesh against real
  neighbours, `load` being `keep` grown by a Cell.
- **Between levels**, there are no skirts and no stitching geometry. The
  strategy is _overlap plus coverage priority_. The coarse level extends one of
  its Cells under the fine level for any Eye position (decision 4), so a place
  where the fine level's boundary face was culled against a solid neighbour it
  did not draw has the coarse surface behind it, and the viewer, who is always
  on the fine side, sees the coarse surface and not the sky. The finer level's
  camera wins everywhere it has geometry (decision 8). What is left is a step
  of at most one coarse Voxel where two heights disagree, which is the LOD's own
  resolution and is constant in screen space by construction of the ring
  widths.
- **Pinned by a test, not by a screenshot alone.** Over a sweep of columns, the
  coarse level's top solid Voxel, in metres, is within one coarse Voxel of
  `Worldgen::surface_height_m` and no more than half a Voxel below it, and the
  top is the grass cap in most columns. That is the bound the "step of at most
  one coarse Voxel" rests on. The upper bound is loose by construction (the top
  rounds up); a downsample that lost the surface Voxel breaks the lower one, and
  one that broke a Material tie by id breaks the grass count.
- **Where the strategy fails**, the fallback is a skirt on the coarse level's
  hole boundary, one coarse Voxel tall, emitted by the Client from the mesh it
  already has. It is not built unless the screenshot at ticket 08 shows a crack;
  the spec names it so that the answer to a crack is not to widen the overlap
  until it is a performance bug. Ticket 08 shows none once the overlap holds
  for every Eye position, so it is not built.

### 10. What a Mod registers or parameterises

Nothing per Voxel, so ADR-0001 holds by construction. A Mod may set two numbers
in the same `Copy` data that `WorldgenParams` is, once at load: the full-detail
radius and the reach. Both are metres. The downsample has no hook and no
callback: a Mod's new Materials and Shapes take part through the tables they are
already in (`volume` for a Shape, an id for a Material), and a Material's
behaviour at distance is whatever "dominant by volume, kept as itself" gives it.
Neither number is World save data: like `StreamingSettings` it is how far the
Server streams, not what the World is.

### 11. Everything physical stays in metres

Distances are `view_distance_m`, `lod_distance_m`, `hysteresis_m` and the
camera's `near` and `far`. A Cell count is derived from them per level and no
Voxel count appears. The only Voxel-unit quantity is the half-volume threshold
`8 * 4096 / 2`, which is a fraction of a Voxel and so dimensionless.

## Invariants, and where each is enforced

1. **The Server is the sole authority over World state.** `LodStore` and
   `LodState` are `pub(in crate::server)` like `ChunkStore`; the Client receives
   `LodMeshReady` and `LodUnloaded` and nothing else; no `client/` file imports
   them. The Client's camera refactor adds a `MainCamera` marker and nothing
   that reads the Server.
2. **No Mod reaches `tesserae-core` directly.** Decision 10. No host call is
   added.
3. **`tesserae-core` gains no dependency.** One new module, `lod.rs`, over
   `Chunk` and `ShapeTable`. `./scripts/cargo-ci`'s Boundaries step runs.
4. **Everything physical is in metres.** Decision 11, and the level-k Voxel size
   is `voxel_size * 2^k`, never a bare shift of an index.
5. **A job holds Arcs and returns an owned value.** No coarse stage function has
   a `LodStore`, `ChunkStore` or `LodState` in its signature.

## Performance

`cargo bench -p tesserae-core --bench meshing -- --save-baseline before` on
unmodified `main` is saved **before any code change**, then compared after. No
existing hot loop changes: `mesh.rs`, `light.rs` and `chunk.rs` are untouched,
so the expected delta on every existing benchmark is nil and any other result is
a measurement artefact to be explained, not accepted.

- **Load.** Wall-clock benchmarks lie when other sessions run. The load average
  is read before and after each run and quoted. A comparison that matters
  (anything within 5%) is rerun as an interleaved A/B against a build of `main`
  in its own worktree, minimum of several runs, never through a fixed `/tmp`
  name.
- **New benchmarks**, in a new `benches/lod.rs` so that `meshing.rs` is not
  edited: `lod/downsample` over terrain, uniform and stair-heavy children;
  `lod/coarse_chunk` (eight generator calls and the downsample, which is what a
  Job costs); `lod/mesh_coarse` (a downsampled terrain Chunk meshed with the
  synthetic light). These have no baseline and are reported in absolute terms
  against the level-0 figures (`worldgen/chunk` 98 us, `mesh_chunk` 342 us on
  the M5 at step 5).
- **Per-ring budget, measured per level**, which DESIGN.md's open item has never
  had. For each level 1 to 5: Chunks held, Chunks non-uniform, Chunks meshed
  non-empty, generation and mesh microseconds per Chunk, quads and vertex bytes
  per mesh, and time to fill that level alone. Plus the total vertex memory at
  the default reach, because 4 km is the first time that number can be large.
- **Level 0 must not pay.** The `dev` harness (`scripts/frame-runs`) is run with
  `TESSERAE_LOD=0` and `=1`, interleaved, and level 0's fill time and the worst
  tick are compared. `MainThreadCost`, not frame time, is the figure, for the
  reason ticket 06 of the off-thread spec gives.

## Known loose ends, declared rather than discovered

- **Edits are visible at levels 0 and 1 only**, and last only while their
  level-0 Chunk is held. Both change with a save step; the coarse levels then
  read it.
- **Emitters vanish at distance.** See decision 2. The cheap fix is known.
- **Sky light inside a cave or under an overhang is full at coarse levels.** See
  decision 3. A column rule in the generation job fixes it without touching the
  flood.
- **No blended draw call**, so coarse water and glass are built and not drawn,
  as at level 0.
- **Thin features vanish at the next level.** A one-Voxel wall or pane is under
  half a coarse Voxel. Inherent to a mip.
- **One FIFO pool serves every stage of every level.** Decision 5 names the
  measurement that would justify a second.
- **No distance fog or atmosphere.** A 4 km view against a flat clear colour has
  a hard horizon; fog is step 9's look-pack work and not LOD's.
- **The `dev` gallery and builders** place at level 0 and are unaffected.
- **Ring widths are three of their own Cells** (`view_distance_m * 2^(k-1)`
  wide). A World that wants a sharper falloff changes the radius, not the
  structure.

## Done when

`cargo dev` shows LOD rings rendering to the 4 km target without visible seams,
with a screenshot taken and shown, including one with the debug tint so the
rings are visible as rings; `./scripts/cargo-ci` passes in full; the benches
show no unexplained regression against the saved baseline and the new ones are
reported; the per-level budget table is in `StreamingBudget`'s doc comment and
DESIGN.md's open item; and DESIGN.md, `docs/architecture.md` and `CONTEXT.md`
are updated. If part of the scope is blocked, the rest is finished and the PR
says exactly what was left out and why.

## Outcome

Written as the tickets finish; what changed from the text above, and why.

### Ticket 08: seams

Ticket 06's captures showed three defects, all in the rings and none in the
shader, and a fourth idea that did not survive.

- **The hole's half-height was 0**, so every coarse level drew the terrain above
  and below the Eye in the same column as the next level, and the higher
  surface won per pixel: patchwork, worst from directly above. The hole is now
  as tall as the finer level's rows allow for either anchor parity (decision 4).
- **The overlap was measured from the anchor Chunk's centre**, not the Eye, so
  it was a Cell on one side and none on the other: a straight seam when the Eye
  sat on a lattice line. The hole is now the largest cylinder that, grown by one
  Cell, fits the finer disc for all four parities. At the defaults it is radius
  1 and half-height 1 against 6 and 3, where the first arithmetic gave 2 and 0.
  A test walks every Cell of both levels for every parity instead of using the
  rule's own shortcut, and pins that no wider or taller hole also fits.
- **Hairline cracks at ring joins** went with the overlap: with the coarse level
  under the fine one for any Eye position, a culled boundary face has a surface
  behind it. The skirt of decision 9 was not built.
- **The depth push (`BIAS_FRACTION`) is gone.** It could not work (decision 8):
  a coarse surface up to a coarse Voxel above the fine one is `dh / sin(angle)`
  further along the ray, far more than a fraction of a Voxel at a grazing view.
  One camera per level, coarsest first, gives coverage priority instead, so
  there is nothing to tune. The far cameras' `near` stays at a quarter of
  `view_distance_m`: at 48 m no coarse peak filled the view in any capture, and
  nothing larger was needed.
- **Two suns.** A light's layers are also its shadow-caster mask for every view
  it lights, so one sun on every layer rendered level 0 into every coarse
  camera's cascades. The shadowed sun is on layer 0 only and a plain one on the
  coarse layers, so coarse terrain neither casts nor receives a shadow.

**Pinned by tests.** Over 60 random columns at each of levels 1 to 5 across
the 4 km square, the coarse top Voxel is within one coarse Voxel of
`Worldgen::surface_height_m` and never more than half a Voxel below it, and at
least 70% of tops are grass. Mutation-checked against the Voxel rules, all
three killed: dropping the height tie-break fails the grass count (28 of 60 at
level 1), a solid threshold of three quarters fails the lower bound (level 3),
and one of a quarter fails the upper bound.

**What the captures show** (Apple M5, `TESSERAE_SCREENSHOT_OFFSCREEN=1`, 1080p;
not committed): the horizon at the default vantage and at 150 m, plain and with
`TESSERAE_LOD_TINT=1`, and a straight-down view. Rings are bands to a flat
horizon with no patchwork and no coarse bump through the fine layer, and a ring
join at full resolution is a step of a Voxel and no gap. A magenta clear colour
(`TESSERAE_SCREENSHOT_MAGENTA_SKY`) over five views, counting clear-colour
pixels under terrain in each column, found none that were not a silhouette
edge, with the Eye on a lattice line (the old straight seam), at 30 degrees, at
150 m, at 300 m and straight down. Reach: the furthest drawn level-5 Chunk
starts 4061 m out and ends 5174 m out, so terrain is drawn past 4096 m.

**What the extra cameras cost.** Median flight frame over five interleaved runs
each, offscreen, load 4.5 to 5.9: 3.9 to 4.1 ms with ticket 06's two cameras
and 5.4 to 5.9 ms with one per level (one run of each build ran at the 60 Hz
cap, 16.6 ms, and is left out). Four more cameras for 1.5 ms is about 0.4 ms
each. The worst tick is unchanged at about 1 ms.

### Ticket 07: the budget per level

Apple M5, `dev` build, offscreen, 1080p, load average 2 to 7. The caps are swept
with `TESSERAE_LOD_BUDGET` (the `dev` build only), interleaved, and
`TESSERAE_LOD=0` is the no-coarse row. A run whose median flight frame is 16.6
ms hit the 60 Hz cap and is left out, which was about half of them, so a row is
3 to 23 runs; the rest are medians.

**Per level**, over five runs with the Eye held still, the defaults. A
level's fill time is when it has held its steady count while the others fill
too, not alone: the six share one pool.

| Level | Held | Non-uniform | Meshed (non-empty) | Generate | Mesh   | Quads   | Bytes   | Filled |
| ----- | ---- | ----------- | ------------------ | -------- | ------ | ------- | ------- | ------ |
| 1     | 2167 | 386         | 776 (221)          | 698 us   | 611 us | 135 813 | 25.0 MB | 3.0 s  |
| 2     | 2167 | 394         | 776 (216)          | 615 us   | 553 us | 125 800 | 23.2 MB | 5.1 s  |
| 3     | 2167 | 394         | 776 (216)          | 519 us   | 511 us | 119 200 | 21.9 MB | 7.2 s  |
| 4     | 2167 | 394         | 776 (216)          | 371 us   | 496 us | 70 930  | 13.1 MB | 9.4 s  |
| 5     | 1243 | 226         | 328 (73)           | 371 us   | 488 us | 17 760  | 3.3 MB  | 10.6 s |

Total vertex and index memory is 127.9 MB at the default reach, 41.5 MB of it
level 0's and 86.4 MB in the coarse levels. `lod/coarse_chunk` costs 2.14 ms at
the surface of level 1 falling to 0.79 ms at level 5, and an air or rock Chunk
costs 51 to 72 ns; `lod/mesh_coarse` costs 0.39 ms falling to 0.30 ms. Most jobs
are early-outs, so the mean Job is about a third of a surface Chunk.

**The caps**, medians:

| In flight / landed (gen, mesh)   | level 0 shown | levels filled | worst tick |
| -------------------------------- | ------------- | ------------- | ---------- |
| no coarse levels                 | 1.33 s        | -             | 0.8 ms     |
| 8, 16 / 8, 8                     | 1.46 s        | 20.8 s        | 1.05 ms    |
| 16, 32 / 16, 16 (the defaults)   | 1.66 s        | 11.1 s        | 0.9 ms     |
| 24, 48 / 24, 24                  | 1.75 s        | 8.0 s         | 0.8 ms     |
| 32, 64 / 32, 32                  | 1.96 s        | 6.4 s         | 1.0 ms     |
| 64, 128 / 64, 32                 | 2.45 s        | 4.6 s         | 1.2 ms     |
| 256, 512 / 512, 128              | 3.61 s        | 3.1 s         | 1.2 ms     |

There is no free row: each buys the distance with level 0's time, because the
pool is one FIFO of three workers and level 0's fill keeps them all busy. The
defaults are the first row where level 0 is 0.3 s late and no more. The landing
caps do not bind (64 / 64 landed fills in the same 11.0 s), nor does a smaller
mesh cap (16 in flight is 11.3 s). The worst tick is 0.8 to 1.2 ms in every row,
so a bad cap costs fill latency and not frame time, as ADR-0007 says.

**A second pool for coarse work: no.** Six async threads instead of three bring
level 0's shown time back to its uncontended value (1.35 s against 1.37 s) and
raise the worst tick from 0.95 ms to 1.6 ms, eight runs each, interleaved. The
cost is CPU time on the same cores, which a second pool does not create. One
pool, and the defaults stay at 16 / 32 in flight and 16 / 16 landed, with the
reasoning now in `LodBudget`'s doc comment.

What was not measured: the fill of one level alone, which would need a way to
switch the others off, and any load above 7, where the 60 Hz cap makes frame
figures useless.

### Ticket 09: benchmarks against main

`cargo bench -p tesserae-core --bench meshing -- --baseline before`, with the
baseline taken on unmodified `main`. The baseline was in the branch worktree's
`target/criterion` and not in the worktree this was run from, so the run used a
copy of it as `CRITERION_HOME`. Apple M5, load average 3.3 to 4.3, one run of 51
benchmarks (8 min 44 s): 33 improved by up to 14%, 11 are within noise and 7
regressed, six of them by under 2%. No existing source file in `tesserae-core`
changed (the diff is `lod.rs`, `benches/lod.rs`, two lines in `lib.rs` and a
`[[bench]]` entry), so the improvements are the host at a different time from
the baseline's. The outlier, `snapshot/make_mut_shared` at +11.7%, is a 370 ns
benchmark whose own interval is 353 to 400 ns.

The small regressions are real, though, and reproduce. Interleaved against a
build of `main` in its own worktree, three runs each (four for the second pair),
medians:

| Benchmark                  | main      | branch    | ratio |
| -------------------------- | --------- | --------- | ----- |
| `mesh_chunk/straddling`    | 390.3 us  | 396.9 us  | 1.017 |
| `light_chunk/leaves_canopy` | 169.4 us  | 170.8 us  | 1.008 |
| `greedy_mesh/terrain`      | 323.2 us  | 337.6 us  | 1.045 |
| `snapshot/make_mut_shared` | 367 ns    | 409 ns    | 1.11  |

**Why.** None of these runs code that changed. Building the branch with the one
`pub mod lod;` line and its re-export removed gives `main`'s figures exactly
(`greedy_mesh/terrain` 1.004, `mesh_chunk/straddling` 0.999), so the cost is the
module's presence and not anything it does. The bench profile has 16 codegen
units and no LTO, so a new module re-partitions the crate and moves hot
functions between units; with `codegen-units = 1` on both sides the gap closes
(`mesh_chunk/straddling` 1.001, `greedy_mesh/terrain` 1.02, inside that
benchmark's 3% spread). That is the layout sensitivity step 4 already met. It is
not fixed here: it is under 5% on a path that costs 340 us a Chunk and runs off
the main thread, and a profile change would be a decision of its own.

**The new benchmarks**, absolute, since they have no baseline:
`lod/coarse_chunk` is 2.14 ms at the surface of level 1, 1.30 ms at 2, 1.43 ms
at 3, 0.97 ms at 4 and 0.79 ms at 5, and 51 ns for an air Chunk and 72 ns for a
rock one at level 1; `lod/mesh_coarse` is 0.39, 0.35, 0.34, 0.32 and 0.30 ms.
Against the level-0 figures (`worldgen/straddling` 168 us,
`mesh_chunk/straddling` 397 us) a coarse Chunk costs more to generate, because
eight generator calls feed one downsample, and about the same to mesh.

## Tickets

| #   | Title                                  | Blocked by |
| --- | -------------------------------------- | ---------- |
| 01  | The downsample in core                 | -          |
| 02  | Levels, shells and the hole            | -          |
| 03  | Coarse generation jobs and the store   | 01, 02     |
| 04  | Coarse meshing and delivery            | 03         |
| 05  | Edits invalidate the level-1 cover     | 04         |
| 06  | The Client draws levels, in two layers | 04         |
| 07  | Size the budget per level              | 04, 06     |
| 08  | Seams and the 4 km capture             | 06, 07     |
| 09  | Docs, ADR-0011 and the PR              | 05, 08     |
