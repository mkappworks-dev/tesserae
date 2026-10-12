# Shaped voxels (second mesher pass, per-shape collision and opacity)

Build-order step 5. Greedy meshing merges coplanar faces of identical full
cubes, and a stair, slab or fence cannot merge with anything, so the mesher
becomes two-pass: greedy-mesh the full cubes, then append per-Voxel geometry for
shaped Voxels. DESIGN.md's "Shaped voxels" section lists four consequences.
Three are this step's work; the fourth (LOD) is step 6's, but its answer is
decided here.

1. A Voxel is (Material, Shape, Orientation). True since step 1.
2. Collision becomes per-Shape.
3. Light propagation needs per-Shape opacity.
4. LOD must decide what a stair becomes at half resolution.

## Vocabulary this step adds

Three terms, to be added to `CONTEXT.md` in the same PR, plus a sharper
definition of **Orientation**.

- **Nodebox** - an axis-aligned box inside one Voxel, in sixteenths of the
  Voxel's edge. A Shape is a union of them. Luanti's word, kept because
  DESIGN.md already promises "Luanti-style nodeboxes".
- **Sealed face** - a face of a Voxel that its Shape covers completely. Sealing
  is what occlusion and light both ask about, and it is derived from geometry,
  never authored.
- **Edit** - a change to one Voxel, applied by the Server. The Client sends an
  intent; it never makes an Edit.
- **Orientation** (sharpened) - one of the 24 axis-aligned rotations of a Shape.
  Meaningless for a full cube, which looks the same under all of them.

## Decisions

### 1. A Shape is a row in a table, not a variant of an enum

`ShapeId` is already a `u8`, and DESIGN.md puts nodeboxes in scope, which a Mod
will want to declare. A Rust enum cannot grow at load time and a Mod cannot add
a variant to it, so a Shape is data: a `ShapeTable` in
`tesserae-core/src/shape.rs`, indexed by `ShapeId`.

```rust
pub struct NodeBox { pub min: [u8; 3], pub max: [u8; 3] } // 0..=16
pub struct ShapeDef {
    pub boxes: Vec<NodeBox>,            // what you see
    pub collision: Option<Vec<NodeBox>>, // None = the same boxes
}
impl ShapeTable {
    pub fn builtin() -> Self;
    pub fn register(&mut self, def: ShapeDef) -> Result<ShapeId, ShapeError>;
}
```

Sixteenths because they are exact: every coverage test is an integer comparison,
and no float epsilon decides whether a hole appears. A box is a fraction of a
Voxel, so it is dimensionless and ADR-0003 does not touch it; anything that
turns a box into a distance does so by multiplying by `voxel_size`.

Built-ins, with ids that are **permanent**: 0 full cube (reserved, and shape 0
can never be redefined, because pass 1 and the Chunk fast paths check for the
zero shape without a table), 1 slab, 2 stair, 3 post (a fence post without its
arms). Canonical orientations: the slab fills the lower half; the stair has its
tall side at +Z; the post is a centred pillar, 4/16 square and full height.

What step 7 fills in: the mod loader calls `register` once per declared Shape in
a batch at load, never per Voxel, so ADR-0001's "no per-Voxel host call" holds
by construction - the WASM side hands over a list of `ShapeDef`s and everything
after that is a native array lookup. Ids 4 and up go to Mods, handed out in
registration order, so a World needs a name-to-id registry in its save data.
That makes the id-to-definition mapping save data, which is the reason for
ADR-0006.

Not in this step: shapes whose geometry depends on neighbours (a fence growing
arms towards other fences - Luanti's `connects_to`), and collision boxes taller
than the cell (a fence you cannot jump). Both need the mesher or the sweep to
look at a neighbour, which is a different table shape. A Voxel whose ShapeId the
table does not know is invisible and intangible rather than a panic: it is what
a World saved with a Mod looks like when the Mod is missing.

### 2. Orientation is one of 24 rotations, and one encoding serves every Shape

The Voxel's 8 orientation bits hold `up * 4 + turn`, 0 to 23, and the top three
bits stay zero and reserved. `up` is which way the Shape's own +Y ends up (+Y,
-Y, +X, -X, +Z, -Z in that order, so the identity is byte 0), and `turn` is a
quarter-turn count about the new up axis. Values 24 and above are invalid: a
read treats them as the identity, and an Edit carrying one is rejected.

This answers the slab-versus-stair question by not distinguishing them. A slab's
up/down is `up` being +Y or -Y; a stair's four facings are `turn` with `up` at
+Y; an upside-down stair is `up` at -Y; a wall-mounted slab is `up` at +X. All
24 are valid for every Shape and a Shape with symmetries simply has duplicates,
which cost nothing because the table is indexed rather than searched.

### 3. Everything else is derived from the boxes, once, at registration

`register` rasterises a Shape's boxes into a 16x16x16 bit grid, and for each of
the 24 Orientations rotates that grid and derives, from the rotated grid and
nothing else:

- **coverage**: for each of the six faces, a 16x16 mask of which sub-squares of
  the face plane the solid touches;
- **seals**: a 6-bit mask, the faces whose coverage is all ones;
- **quads**: the exposed surface of the grid as greedy-merged rectangles on 1/16
  planes, which is exactly the geometry pass 2 emits, with internal faces
  between boxes already gone;
- **volume**: the count of set sub-cells out of 4096, which LOD uses.

One derivation means one source of truth. A rotation bug is a bug in one
function and the property tests find it for every Shape at once, rather than
three tables (visual, occlusion, light) drifting apart. The collision boxes are
the one thing not taken from the grid: they are the declared boxes, rotated by
the same matrix, because a collision shape may legitimately differ from the
visible one (a pressure plate you walk through is `collision: Some(vec![])`).

The cost, measured, is about 1.2 ms per Shape (`shape_table_builtin` is 4.8 ms
for the four built-ins), paid once at load: roughly a third of a second if a Mod
registers all 255. Orientations whose rotated grids come out identical could
share a derivation if that ever matters; it does not today.

### 4. Occlusion is per face, by coverage, and never culls the unproven

This is the change most likely to put a visible hole in the world, so the rule
is stated so that it cannot, and a test checks it against ground truth.

A face emitted on a Voxel's boundary plane is culled if and only if the Voxel
across it covers that part of the plane. Pass 1 emits whole 1x1 faces, so it
asks whether the neighbour's opposite face is **sealed**. Pass 2 emits
rectangles, so it asks whether the rectangle is a subset of the neighbour's
**coverage** mask. A neighbour that covers less than the rectangle never culls
it, so the failure mode is some hidden overdraw, never a hole. A floor of slabs
relies on the subset test, not the seal: a slab's side face is half a plane, the
neighbour slab's matching side covers exactly that half, and so a slab floor's
interior faces all cull. A seal-only rule would keep four hidden quads on every
slab of the floor.

The hole-free property is tested by brute force: build random small scenes of
cubes and shaped Voxels, compute the exact set of exposed sub-cell faces from a
1/16-resolution occupancy grid of the whole scene, mesh the scene, rasterise
every emitted quad back to the same sub-cell faces, and assert that every
exposed face is covered by a quad with the matching normal. That is the check
that would have caught a wrong rotation, a wrong plane, or a culling rule that
is too eager, and it runs over hundreds of seeded scenes across all 24
Orientations.

### 5. Light: per-face sealing, and no authored light-reduction value

`blocks_light` is binary today, and the question is what replaces it. Options
considered:

- A per-Shape opaque or transparent bit. A stair roof would let all the sky
  through, which is plainly wrong and the first thing a player builds.
- Minecraft's opaque/transparent plus a per-block reduction number. The
  reduction is a property of what a Voxel is made of (water, leaves), not of its
  geometry, so it belongs with Materials in steps 7 and 8, and adding it to
  Shapes would be a place for the two to be confused.
- **Per-face sealing, derived from geometry.** Chosen. It needs no authored
  data, it makes a stair block the light that a stair should block, and it is
  close to what Minecraft's light engine does for blocks that opt in to
  shape-based occlusion.

The rule: light travels from Voxel A to the adjacent Voxel B across their shared
face unless **B seals its face towards A**, or **A is shaped and seals its face
towards B**. A full cube is not asked the second question: a cube only ever
holds its own emission, which leaves freely. That asymmetry is what lets
`LANTERN`, a fully sealed cube, light its surroundings, and it is the existing
step-4 behaviour kept rather than a new rule.

Both solvers share the rule through one function so they cannot drift:

- `light_chunk` replaces its `opaque: Vec<bool>` with a `Vec<u8>` of seal bits
  per Voxel, and the flood tests the destination's entry bit and, for shaped
  sources, the source's exit bit. Full cubes and air are decided without
  touching the table.
- `seed_from_border` has to know whether the **neighbour's** Voxel across the
  plane is a shaped one with a sealed face, because a stair beside a Chunk
  boundary holds a non-zero light value and would otherwise leak through its
  back into the next Chunk: a seam in the light that exists only where Chunks
  happen to meet, and that the incremental solver, which sees both Voxels, would
  not reproduce. So `light_chunk`'s border changes from `[Option<&ChunkLight>;
  6]` to `[Option<LitChunk>; 6]` - the pair the mesher already takes. The
  neighbour's Voxels are only read when its Chunk `may_contain_shaped()`, a
  palette scan of tens of entries, so terrain pays nothing.
- `relight`'s brighten pass tests the same edges. Its darken pass deliberately
  traverses every edge, sealed or not: it clears what the old Voxel could have
  lit, over-clearing is repaired by the regrow pass (every cleared region ends
  at a strictly brighter Voxel, which seeds it), and under-clearing would be the
  bug. A comment records this because it looks like the one pass that forgot
  opacity.

Two fast paths in `light_chunk` need a new guard. The open-sky shortcut is exact
only if nothing above can block the descent, so it now also requires that the +Y
neighbour `may_contain_shaped()` is false; a bottom slab at the base of the
Chunk above holds sky 15 in its cell and seals the way down. The opaque-Chunk
shortcut is unchanged, since a uniform full cube is still the only thing that
qualifies.

Two partial shapes that only **jointly** cover a face (a top slab beside a
bottom slab) were first left to leak, pinned by a test. They no longer do: the
edge is also blocked when the two facing coverage masks together cover the plane,
tested only for pairs of shaped Voxels. See
[joint coverage in the light edge rule](../2026-10-12-joint-coverage-light/spec.md).

### 6. Pass 2: per-Voxel geometry, lit per quad

Pass 1 stays as it is, except that its occlusion test asks the table (decision
4). Pass 2 then runs only if the Chunk's palette holds a shaped entry, so
terrain skips it entirely and the count loop at the top of `greedy_mesh`, which
today reads all 32768 Voxels to fill `skipped_shaped`, goes away for it.

For each shaped Voxel it appends the precomputed quads for its (Shape,
Orientation):

- A quad on the cell's boundary plane faces the neighbour, so it is culled by
  the coverage rule and otherwise lit by the neighbour's light, exactly as a
  pass-1 face is. It is "the light of the Voxel in front".
- A quad inside the cell (a stair's tread) faces the same Voxel's own air, so it
  is lit by that Voxel's own stored light. A partly open Voxel does store light
  under decision 5, which a cube does not.
- The light is **per quad and flat**, the same four copies of one `Light` that
  pass 1 writes, so the shader's flat interpolation and the vertex layout are
  untouched. Smooth per-vertex lighting and ambient occlusion are look-pack
  work.
- Pass-2 quads never merge across Voxels; that is what "cannot merge with
  anything" means. The merge key `(MaterialId, Light)` is pass 1's alone.
- UVs are the sub-cell position inside the Voxel divided by 16, so a half-height
  slab side shows the top half of the texture rather than a squashed whole one,
  and a tiling sampler still repeats per Voxel.

`ChunkMesh.skipped_shaped` becomes `shaped_voxels`: the count pass 2 emitted
geometry for. Nothing else about `ChunkMesh` changes, so `chunk_mesh_to_mesh`,
the attribute set, `specialize` and the shader are all untouched. The spare 16
bits of `ATTRIBUTE_MATERIAL` stay unclaimed: nothing in the shader varies by
Shape because the geometry already carries it, and a third attribute would be
bytes per vertex for nothing. The doc comment that reserves them for step 5 is
rewritten to say what would earn them (shape-aware shading in a look pack).

Both `mesh_chunk` fast paths survive. A neighbour's shaped Voxel never emits
into this Chunk, and a Chunk of uniform air emits nothing whatever it
neighbours. A uniform **shaped** Chunk is not a full-cube occluder and does
mesh, as it should.

### 7. The edit path is Server-side, intent-driven and queued

The Client has no read access to World state (ADR-0002, `chunk_view.rs`), so it
cannot pick a Voxel itself, and a position it sent would be a claim the Server
had to re-check. It sends a ray instead, in metres, and the Server finds the
Voxel:

```rust
#[derive(Message)]
pub struct EditIntent {
    pub action: EditAction,
    pub origin: Vec3,
    pub direction: Vec3,
}
pub enum EditAction { Break, Place(Voxel) }

#[derive(Message)]
pub struct EditRejected { pub reason: RejectReason }
```

Both live in `server/edit.rs`: the Server is the side that decides what intent
it accepts, which is why `StreamingAnchor` is declared in `server/streaming.rs`
too.

**Messages written by the Client can be dropped** (`server/mod.rs` explains
why): a `MessageReader` in `FixedUpdate` misses a message written in a frame
that is followed by a frame running no tick, which at 120 fps against a 64 Hz
tick is a large fraction of clicks. So a Server system in `RunFixedMainLoop`'s
`BeforeFixedMainLoop`, which runs every frame, drains the messages into a
Server-private `PendingEdits` queue, and the tick applies from the queue. The
queue is also the shape a network transport will fill in step 10. `EditRejected`
goes the other way, Server to Client, which never drops.

The tick applies up to `EDITS_PER_TICK` queued edits, before `stream` so the
same tick's `publish` sees them. An edit whose Chunk is not loaded is rejected;
one whose Chunk is still `unlit` is deferred a tick at a time and dropped after
64, because the incremental relight and the bulk solve must not both write the
same Chunk (the same reason the lightbox waits).

`apply` does, for one Voxel: resolve the ray with `collide::raycast` (decision
9) up to `EDIT_REACH_M`, validate, `set_voxel`, `relight` through
`LightLattice`, and mark Chunks dirty. It marks the edited Chunk, every Chunk
the relight wrote to, **and the neighbour across any boundary plane the Voxel
sits on** - the third is the one the lightbox never needed. Removing a stone
Voxel in the dark changes no light, and the neighbouring Chunk's mesh still has
a face to grow. This is the function the `dev` lightbox and gallery call, so
there is one edit path and the dev tools exercise the real one.

Validation, in order: the Shape is registered; the orientation is valid; the
Material is not air (a Material registry is step 7's); the ray hits within
`EDIT_REACH_M`, which is in metres; the target Voxel for a Place is air; the
Chunk is loaded; and the Chunk would not exceed the shaped cap (decision 11). An
Edit that replaces a shaped Voxel with another shaped Voxel is net zero against
the cap.

`ChunkStore` gains a read-only `VoxelVolume` impl (the trait `collide` takes,
decision 9). With the edit path live, the `cfg_attr(not(feature = "dev"),
allow(dead_code))` attributes on `VoxelCoord`, `LightLattice`, `set_voxel`,
`light_at`, `MeshState::edited` and `is_unlit` come off. Any that still have
only `dev` callers stay, and the PR says which.

### 8. The Client places and breaks, and shows what it holds

The least that makes the edit path real: with the cursor grabbed, left click
breaks and right click places, from the camera's centre ray; 1 to 5 choose what
is held (stone cube, slab, stair, post, lantern); `Q` steps `turn` and `E` steps
`up`; a crosshair and a line of text say what is held and show the last
rejection. The click that grabs the cursor is not also an edit. Keys avoid the
camera's own (WASD, Space, Ctrl, Shift, R, Esc).

### 9. Collision lands, as a sweep and a ray, both in metres

There is no player controller, so a sweep has no caller in the app. It lands
anyway, for two reasons. A Shape table that has never been asked a collision
question is unproven, and a rotated nodebox is exactly where an off-by-one
hides; the cheapest honest consumer is the sweep itself, with tests that stand a
box on a half slab. And picking needs the ray half now. Splitting the sweep out
was the alternative and it is not much of a saving: it is a modest amount of
code over the box data this step has to build anyway.

`collide.rs` is the one module that asks the grid a geometric question, over a
trait the Server implements on its store, so core keeps its zero dependencies:

```rust
pub trait VoxelVolume { fn voxel(&self, at: [i32; 3]) -> Option<Voxel>; }
pub fn sweep(vol: &impl VoxelVolume, shapes: &ShapeTable, voxel_size: f32,
             aabb: Aabb, delta: [f32; 3]) -> Sweep;
pub fn raycast(vol: &impl VoxelVolume, shapes: &ShapeTable, voxel_size: f32,
               origin: [f32; 3], dir: [f32; 3], reach_m: f32) -> Option<RayHit>;
```

Everything is in **metres**, with `voxel_size` the single conversion, so a
half-height slab is 0.5 m at 1 m Voxels and 0.125 m at 0.25 m Voxels. `sweep`
resolves the three axes in turn against the rotated collision boxes of every
Voxel the box can reach, and reports the distance actually moved and which axes
were blocked - the result a controller needs for "on the ground" and for
sliding. `raycast` walks cells with a DDA and tests the ray against each cell's
boxes, so it enters a stair's notch rather than stopping at the cell, and
returns the cell, the face entered and the distance.

Out of scope, deliberately: step-up and step height (a controller policy, and
the number is in metres when it arrives), gravity, entities, and a `None` Voxel
(an unloaded Chunk) is solid to a sweep so nothing falls out of the loaded
world.

### 10. A stair at half resolution is a full cube or nothing, by volume

DESIGN.md says "usually a full cube, or nothing" and the table makes "usually"
computable. A coarse Voxel aggregates 2x2x2 finer ones. Each child contributes
its Shape's **volume** (out of 4096; air 0, a cube 4096), and the coarse Voxel
is a **full cube of the dominant Material if the children's volumes sum to at
least half of the eight**, otherwise air. So a stair (3072) and a slab (2048)
read as solid and a post (about 4/16 x 4/16 of the footprint) as nothing. Ties
go to solid: a slab floor that vanished at distance would be a far worse
artefact than one that thickened.

Consequences, all of which make step 6 simpler: LOD level 1 and above never
contain a shaped Voxel; pass 2 never runs there; and "the mesher does not need
to know which level it is meshing" survives. This step provides
`ShapeTable::volume` and a test that pins the three built-ins to the rule; the
downsample itself is step 6. The PR template's "LOD downsample behaviour
defined" is this paragraph.

### 11. Shaped Voxels stay sparse by a cap, enforced at the sources

DESIGN.md: "Keep shaped voxels sparse in worldgen and cap them per chunk, or the
mesher's fast path stops mattering." The cost is real: a shaped Voxel emits up
to about a dozen unmerged quads where a cube merges into almost nothing, and a
Chunk of 32768 stairs is hundreds of thousands of quads.

- **Worldgen places none in this step.** It stays byte-for-byte what step 4
  shipped, so every bench input that comes from it is unchanged, and a test pins
  `shaped_count() == 0` over a sweep of generated Chunks as the contract: a
  later generator that wants slope-smoothing stairs budgets under the cap.
- `MAX_SHAPED_PER_CHUNK` lives in core, and is 1024 (3% of a Chunk). The rule
  that sized it: a Chunk **at** the cap meshes within +50% of
  `mesh_chunk/straddling`, because a tick has twelve meshes in it. Measured,
  terrain with a stair on top of every column (1024 of them) takes 527 us
  against 350 us for the terrain alone, which is 41%; a Chunk of nothing but
  stairs takes 5.2 ms. Pass 2 is linear at about 170 ns a stair, so 2048 would be
  about +95% and fails it.
- The **edit path** enforces it. At the cap a Place is rejected with
  `RejectReason::TooManyShaped` and the Client says so. Rejecting is chosen over
  silently placing a cube, which would change what the player built without
  telling them. `Chunk::shaped_count()` scans only if `may_contain_shaped()`, so
  it costs nothing on a Chunk that has never held one and tens of microseconds
  on one that has, once per edit.
- The **mesher does not police** it. A Chunk over the cap is a bug in a source,
  and an over-cap mesh that quietly dropped geometry would be a hole nobody
  could explain.

## Invariants, and where each is enforced

1. **The Server is the sole authority over World state.** `ChunkStore`'s
   mutators stay `pub(in crate::server)`; `server/edit.rs` is inside that
   boundary, the Client only writes `EditIntent`, and `VoxelVolume for
   ChunkStore` is read-only. A review check: no `client/` file may import
   `ChunkStore`.
2. **`tesserae-core` gains no dependency.** The sweep takes a trait, the Server
   implements it. Nothing here needs `glam`: positions are `[f32; 3]` and `[i32;
   3]`, as in step 4.
3. **No Mod reaches `tesserae-core` directly.** The `ShapeTable` is built for
   one batched `register` per Shape at load and read natively afterwards.
4. **Everything physical is in metres.** Nodeboxes are Voxel fractions
   (dimensionless); the sweep, the ray and `EDIT_REACH_M` are in metres; the dev
   gallery is placed by a metre position.

## Performance

Baselines are the table in the handoff (step 4, Apple M5, release). The risks,
in the order they are likely to bite:

- **Pass 1's occlusion test** now asks the table. Cube and air are decided
  inline and only a shaped neighbour reaches the table, but this is the inner
  loop, so `greedy_mesh/terrain`, `/solid` and `/checkerboard` are the numbers
  to watch. Benchmark before believing any rewrite of it, as step 4 learned.
- **`light_chunk`'s flood** gains a seal test per face. `straddling_settled` and
  `flooded_air` are the numbers to watch.
- **`seed_from_border`** gains a neighbour-Voxel read behind the
  `may_contain_shaped` gate.
- **Pass 2** costs nothing on terrain by construction. New benches measure it
  where it applies: terrain with a cap's worth of scattered shaped Voxels, and a
  Chunk that is nothing but stairs.

If a baseline regresses, the PR says by how much and why, or the change is
reworked. The streaming worst tick is re-derived from the new numbers, and
`StreamingBudget::default`'s comment is updated if it moves.

## Known loose ends, declared rather than discovered

- **Terrain contains no shaped Voxel**, so a player sees them only by placing
  them or in the `dev` gallery. Slope-smoothing stairs and slabs are the obvious
  worldgen use and are deferred with the cap in mind.
- **No fence arms, no tall collision**, as in decision 1.
- **Material transparency** (glass, water, leaves) is not a Shape property and
  not in this step. `ShapeTable` answers geometry only.
- **The Client UI is a stopgap.** A held-item selector and a crosshair are
  enough to exercise the edit path and will be replaced by real UI.
- **Ray origin is taken from the intent**, not checked against a player
  position, because there is no player entity. The network step adds that check.

## Done when

`cargo dev` shows shaped Voxels rendered, lit and occluding correctly (a
screenshot from the `dev` capture is taken and shown); `./scripts/cargo-ci`
passes in full; `cargo bench -p tesserae-core` shows no unexplained regression
against the step-4 table, with before/after numbers in the PR; and the streaming
worst tick is re-measured against the 15.6 ms a tick has at 64 Hz.
