# Connected shapes and collision taller than the cell

Two gaps step 5 left on purpose, both from DESIGN.md's "Nodeboxes are in
scope". ADR-0006 names them as inexpressible in the current table:

- **(a) Connected shapes** - a fence growing arms towards neighbouring fences
  (Luanti's `connects_to`). The geometry depends on the neighbours, and the
  `ShapeTable` derives everything once, at registration, from the Shape alone.
  - **(b) Collision taller than the cell** - a fence you cannot jump.
  `collide::clip` only visits cells the body's box overlaps, so a box standing
  above its own cell is invisible to it.

They share one cause (a Voxel's geometry is not a function of the Voxel alone)
and one fix (the table learns two things it did not know), but they are
independent: (b) needs nothing from (a), and the tickets land (b) first.

## Vocabulary this adds

To go in `CONTEXT.md` in the same PR.

- **Connection** - an arm a connected Shape grows towards a face-neighbour
  because of what that neighbour is. A Voxel has up to six, as a 6-bit mask in
  the order of the face constants.
- **Connection group** - one of 32 bits a Shape can belong to and can connect
  to. A property of a Shape, not of a Material.
- **Overhang** - how far a Shape's collision extends beyond its own cell, in
  Voxels, at most one.

## Decisions

### 1. Connections are derived from the neighbours, never stored

The mask is computed where it is needed from the six face-neighbours' Voxels and
is never written to a Chunk. Four ways to hold it were weighed:

| Option                                                          | Verdict                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A. Derived at read time** (Luanti)                            | **Chosen.**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| B. Stored in the orientation byte (Minecraft's blockstate)      | Fits: 6 bits for a mask, and orientation is meaningless for a connected Shape (decision 4). Rejected for what it costs, not for the bits. An Edit would have to rewrite up to six neighbours, one or more in another Chunk, and a Chunk generated beside an already-stored fence has no way to learn it should have grown an arm. It also stores a fact that is a function of other stored facts, so a Mod changing `connects_to` leaves every saved fence stale. Every one of those is a bug that appears only after save and reload, or after a seam. |
| C. Widen `Voxel` to 64 bits                                     | Doubles every palette entry for two shapes' worth of benefit, and is the save-format migration `voxel.rs` says to avoid.                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| D. One `ShapeId` per connection pattern (16 per fence), edited. | Needs no new machinery and eats 16 of 256 ids per fence, and still has B's neighbour-rewriting Edit.                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

So **the Voxel's 32-bit layout does not change and the save format does not
change**. ADR-0006's `material:16 | shape:8 | orientation:8` stands, and so does
its rule that a ShapeId's definition is save data. What is new is that a Shape's
definition now includes its arms and its groups; a Mod that changes them
changes how an existing World looks and corrupts nothing, which is the same
property that makes a missing Mod invisible rather than fatal. This is the
reason for a new ADR (0008) rather than an edit to 0006: it is a decision about
what the bytes do _not_ hold.

The costs, accepted: the mesher, the sweep and the ray each read up to six
neighbours for a connected Voxel. They already read them (the mesher's
`Neighborhood` shell exists for exactly this and `VoxelVolume::voxel` is
random access), and only a connected Voxel pays.

### 2. A connected Shape is declared as data

```rust
pub struct CollisionBox { pub min: [i8; 3], pub max: [i8; 3] } // -16..=32

pub struct ShapeDef {
    pub boxes: Vec<NodeBox>,                    // always present: the post
    pub collision: Option<Vec<CollisionBox>>,   // None = the same boxes
    pub connect: Option<ConnectDef>,
}
pub struct ConnectDef {
    pub group: u32,        // the groups this Shape belongs to
    pub connects_to: u32,  // grow an arm towards a neighbour in any of these
    pub to_sealed: bool,   // ... or towards one that seals the face facing us
    pub arms: Vec<Arm>,
}
pub struct Arm {
    pub face: usize,                            // a face constant
    pub boxes: Vec<NodeBox>,                    // drawn when connected
    pub collision: Option<Vec<CollisionBox>>,   // None = the same boxes
}
```

`to_sealed` is Minecraft's "fences meet walls": the neighbour is any Voxel whose
Shape seals its face towards us, which the table already answers for every
Shape and for cubes without a lookup. `group` and `connects_to` are two `u32`s,
so a Mod's whole connection vocabulary is a few words of data handed over in the
one batched `register` call. A Mod callback for "does this connect?" is
excluded by ADR-0001: it would be a host call per neighbour per Voxel.

**Groups belong to Shapes, not Materials.** An oak fence and a stone wall are
told apart today by being different Shapes, so "oak fences do not meet stone
walls" is expressed by giving them different groups, and costs a `ShapeId` per
variety. Making it a Material property is right and is step 7's: there is no
Material registry yet, and inventing one here would be a guess the registry then
has to honour. Declared as a loose end.

### 3. One predicate, three callers

```rust
impl ShapeTable {
    pub fn connections(&self, voxel: Voxel, around: [Voxel; 6]) -> Connections;
}
```

`around` is the six face-neighbours in face-constant order. Bit `f` is set iff
the Shape has an arm on face `f` and either the neighbour's `group & connects_to
!= 0` or `to_sealed && seals(neighbour, f ^ 1)`. Air, unknown Shapes and an
unloaded neighbour (`None` from a `VoxelVolume`, air in the mesher's shell) have
no group and seal nothing, so they never connect. A non-connected Voxel returns
`Connections::NONE` without reading `around`.

The mesher, the sweep and the ray each gather their six and ask this one
function. That is what keeps a fence from being drawn with an arm you walk
through, the same reason step 5 derives every geometric question from one grid.
A test pins the three together (ticket 05).

The query methods take the result, so forgetting to ask is a compile error and
not a fence that collides as a bare post:

```rust
fn quads(&self, voxel: Voxel, c: Connections) -> &[ShapeQuad];
fn collision(&self, voxel: Voxel, c: Connections) -> &[CollisionBox];
fn selection(&self, voxel: Voxel, c: Connections) -> &[NodeBox];   // decision 9
```

Anything `c` cannot change takes no mask: `seals`, `light_mask`, `volume`, and
`coverage` with the caveat in decision 6.

### 4. A connected Shape never seals, and has no Orientation

Two rules, enforced at registration, that make everything else cheap.

**It seals no face under any mask.** `register` derives every submask of the
arms and rejects the Shape (`ShapeError::ConnectedSeals`) if any seals a face.
So pass 1's occlusion test, the Chunk fast paths, `seals`, `light_mask` and both
light solvers stay mask-free and need no neighbour context. A fence blocks no
light and hides no face, which is what a fence does. A "connected" Shape that
would seal a face is a cube with extra steps; make it a Shape and stop asking it
to connect. This is the decision that keeps ADR-0006's `seals`-based lighting
and pass 1 untouched, and it costs nothing a fence, a wall, a glass pane or a
pipe wants.

**It has no Orientation.** Arms are indexed by world face, and a connected
Shape's orientation byte reads as the identity, as a byte of 24 or more already
does. Rotating a fence is a no-op; a thing that needs rotating (a wall-mounted
pipe end) is a different Shape. Registration derives `2^k` variants for `k`
arms, instead of `24 x 2^k`: a fence is 16 derivations, about 1 ms, where
oriented arms would be 24 times that.

**An arm touches only its own face's boundary plane.** An arm on face `f` may
reach plane `f` and must stay off the other five, rejected otherwise
(`ShapeError::ArmTouchesOtherFace`). It is what makes decision 6 exact with one
neighbour read instead of two hops, and no arm that grows from a post needs to
break it.

### 5. The table holds a variant per mask

`Variant` is unchanged for every existing Shape. A connected Shape registers its
24 orientation slots as copies of its bare variant (mask 0), so `variant()`,
the hot path under `seals`, is the same expression it is today. Its other masks
live in a second array reached by one extra index, only on the connected path.
The observable API is decision 3's; the layout is the ticket's to choose
against the benchmark.

Registration derives each mask from the same rasterised grid as everything else
(base boxes plus the arms the mask selects, unrotated), so quads, coverage,
volume and the collision list all come from one source. Selection boxes are
the declared boxes, rotated the way collision already is.

`volume` is the bare variant's. LOD asks "is this Voxel mostly solid" without
neighbours, and a fence is a post and some rails, so it is 'nothing' at half
resolution whichever arms it has; a stair's rule from step 5 decision 10 is
untouched. A chunky connected Shape (a wall) is under-counted at LOD, which is
the safe direction and a known approximation.

### 6. Pass 1 is untouched, and pass 2 culls exactly

Pass 1 asks `seals(across, face ^ 1)`, which is mask-free for a connected
Voxel (decision 4), so its inner loop and its cost are unchanged. This is the
claim the benchmark has to confirm.

Pass 2 handles a connected Voxel by gathering its `around` from the
`Neighborhood` (six `at` reads, all inside the one-Voxel shell the mesher
already has), asking `connections`, and emitting `quads(voxel, c)`. Everything
else about pass 2 stands: boundary quads culled by the neighbour's coverage and
otherwise lit by the neighbour, interior quads lit by the Voxel's own light, one
flat `Light` per quad, no merging across Voxels.

One thing needs more than that. A boundary quad is culled when the Voxel across
it **covers** the rectangle, and a connected neighbour's coverage on its face
towards us depends on whether it has its arm out that way. By decision 4 an arm
touches only its own plane, so that coverage depends on exactly one mask bit,
and that bit is a function of the neighbour's Shape and **us** - the Voxel being
meshed, which we already hold. So `coverage_against(across, face, here)` answers
it with no second hop, and the end caps of two arms that meet cull each other
instead of leaving two hidden quads on every connection.

Falling back to the bare variant's coverage would be conservative (arms only add
solid), costing those two quads per link and never a hole. The exact answer is
one table read more, so it is preferred; if it proves wrong, the fallback is the
safe retreat and the property test's no-hole half is what would say so.

### 7. Collision may extend one Voxel past the cell

A `CollisionBox` is signed and runs `-16..=32` sixteenths, so a box may poke at
most one Voxel out of any side. A fence's collision is `[6,0,6]..[10,24,10]`: a
post 1.5 Voxels high. `NodeBox` (drawn geometry) stays `0..=16`; only collision
may overhang, because only collision is a function that wants to act at a
distance. `MAX_BOXES` bounds both lists as before.

`rotate_box` already works on centred integers, so an overhanging box rotates
into the right axis (a fence lying on its side overhangs sideways). `register`
folds each box's reach beyond the cell into `ShapeTable::overhang() -> [u8; 6]`,
the largest per face in Voxels, 0 or 1. The builtin table's is all zero. A Shape registers under all 24
Orientations, because any may be placed, so one oriented Shape that overhangs
upward overhangs every face of the table and the sweep then looks one cell
further on all six sides (up to 4x4x4 cells for a one-Voxel body, from 2x2x2).
That is accepted for the general case. A connected Shape is unoriented
(decision 4) and so folds only its identity, which is why a fence widens +Y
alone.

Everything physical stays in metres: an overhang is a number of Voxels, so a
1.5-Voxel fence is 1.5 m at 1 m Voxels and 0.375 m at 0.25 m. That is ADR-0003
applied, not worked around, and it means a World of fine Voxels does not get a
human-scale fence from this mechanism (a person is many such Voxels tall). A
Shape scales with the World's Voxel, as a slab does.

### 8. The sweep widens its range by the table's overhang

`clip` visits the cells a body can reach. With overhang it visits more: a body
in cell `y` can touch a box that belongs to the Voxel at `y - 1` whose collision
climbs into `y`. So on each axis the range grows at the low end by the table's
overhang on that axis's positive face (boxes climbing up into us) and at the
high end by its overhang on the negative face (boxes reaching down into us).
With the builtin table both are zero and the loop bounds are byte-for-byte
today's.

Two changes ride along because they are on the path being edited and one is a
cost the new case makes worse:

- `obstacles` returns a borrowed slice. It returns `Vec<NodeBox>` today through
  `to_vec()`, an allocation per visited cell, and a connected Voxel would add
  neighbour reads on top. The table owns the boxes and a static one-box slice
  serves an unloaded cell, so nothing is allocated.
- Neighbours for `connections` are read only for a connected Voxel, behind
  `ShapeTable::is_connected`.

What the sweep does not do: step-up, and so "you cannot jump a fence" is a
statement about geometry (the box top is at 1.5 Voxels) and not about a
controller. A controller's step height is a distance in metres and lands with
the controller.

### 9. Picking uses the drawn shape

`raycast` tests collision boxes today, which was invisible while collision
equalled the drawn shape for every builtin. With overhang it is wrong in a way
you can see: a ray aimed at air above a fence post would strike the invisible
upper half of the collision box, and a pressure plate (`collision:
Some(vec![])`) could not be broken at all.

So `raycast` switches to **selection boxes**: the Shape's declared boxes,
rotated, plus the armed ones of a connected Shape, with no overhang by
construction (`NodeBox` is `0..=16`). That is a behaviour change for any Shape
whose collision differs from its drawing, there are none in the builtin table,
and it is the correct thing to have asked from the start. It is its own ticket
because it is separable and has its own test, and the hole-free oracle needs it
(ticket 02).

### 10. The edit path and streaming need no change

An Edit changes one Voxel and the connected Voxels around it are re-derived at
mesh time, so nothing is rewritten. The dirty rule already marks the edited
Chunk and the neighbour across any boundary plane the Voxel sits on
(`server/edit.rs`), and a mesh job already requires all six neighbours to be
present and settled and lands only if every neighbour it read is unchanged. A
fence's mask is read from exactly those six Voxels, so a placed fence beside
another across a Chunk boundary re-meshes both Chunks by the machinery that
re-meshes them for face culling. Ticket 07 tests it rather than assuming it,
because "the existing rule is enough" is the claim this decision stands on.

The cap: `MAX_SHAPED_PER_CHUNK` counts Voxels and was sized for stairs. A fully
armed fence is several times a stair's quad count, so a Chunk of fences at the
cap might break the rule the cap was sized by (a Chunk at the cap meshes within
+50% of `mesh_chunk/straddling`). Ticket 07 measures it first. If it fails,
connected Shapes cost `ceil(worst-case quads / a stair's quads)` cap units
instead of 1, and the edit path sums units; if it passes, nothing changes. The
bench decides, not the argument.

**Outcome:** it failed. 1024 fences meshed in about 5.2 ms against 1.5 ms for
1024 stairs in the same run, so the fallback applies. The reference fence cost
7 stairs (`ShapeTable::cap_units`, its worst-case quads over a stair's, rounded
up) with two rails per arm and 146 fit in a Chunk. Ticket 09 cut it to one bar
per arm: 4 stairs, 256 fit, and a Chunk of fences takes what a Chunk of stairs
does (547 us against 550 us). Merging collinear faces across Voxels was weighed
and cannot lower the weight; see `.scratch/2026-10-12-merged-rails-across-voxels/`.

Worldgen still places nothing shaped (`shaped_count() == 0` over generated
Chunks is a test), so none of this is reachable from terrain.

Fences are not added to `ShapeTable::builtin()`: ids 0-3 are permanent
(ADR-0006) and ids from 4 are the Mods', so a built-in fence would be a
permanent id taken without a Mod to justify it. The `dev` build registers a
demo fence at startup for the gallery and the hotbar, exactly as a Mod will in
step 7.

### 11. Jobs compute connections from their snapshot

ADR-0007: meshing and light run as off-thread jobs holding `Arc` snapshots, and
the `ShapeTable` is immutable once the World is open (a job holds it by `Arc`;
registering at runtime would need a table epoch). Both hold here. A mesh job
derives a connected Voxel's arms from the `Neighborhood` shell it was handed
(decision 6), never from the live store, and the shell already contains every
Voxel `connections` reads. The sweep and the ray run on the Server's tick over
`VoxelVolume`, not in a job. Connection groups, arms and overhang are fixed at
registration like the rest of a Shape, so no new epoch is needed. Ticket 05
asserts the mesher path takes no `VoxelVolume`.

### 12. The WASM boundary

Both features cross as data: `ShapeDef` grows `connect` and the type of
`collision`, and a Mod hands the table a list of definitions once at load.
Everything after registration is native array lookups and integer tests.
Nothing in this spec is a call per Voxel, per neighbour or per chunk, and the
rule is stated because a "custom `connects_to`" callback is the obvious request
and the one that breaks ADR-0001.

## Invariants, and where each is enforced

1. **The Server is the sole authority over World state.** Nothing here writes a
   Voxel: a connection is derived on the reading side. The Client still only
   holds meshes.
2. **`tesserae-core` gains no dependency.** `connections` takes six `Voxel`s;
   the sweep and the ray gather them through the existing `VoxelVolume`.
3. **No Mod reaches `tesserae-core` directly.** One batched `register` per
   Shape, native afterwards (decision 12).
4. **Everything physical is in metres.** Boxes, arms and overhang are
   fractions of a Voxel and become distances only by `voxel_size`.
5. **A connected Shape seals nothing, is unoriented, and keeps each arm to its
   own plane.** Registration enforces all three, so pass 1, the light solvers
   and the exact culling of decision 6 can rely on them without checking.

## Hole-free property test

`tests/meshing.rs` is the check that would catch a wrong culling rule, so it is
extended rather than supplemented. `random_scene` gains connected Shapes -
a fence, and a six-way pipe so that arms on every face are exercised - placed
among cubes, slabs, stairs and posts, from a test-local table that registers
them. Today's oracle derives the exposed sub-cell faces from
`shapes().collision(v)`, which stops being "the declared boxes" the moment
collision can differ from them. It moves to the declared boxes (ticket 02
introduces them as `selection`), and for a connected Voxel the oracle computes
its own mask from the test's own copy of the `ShapeDef`s and the scene,
**without calling `ShapeTable::connections`**, so the predicate is checked by
something that did not write it.

It keeps both halves: every exposed sub-cell face is drawn, and none is drawn
twice. New alongside it: a line of fences draws no hidden end caps (decision
6), a fence beside a stair draws its arm and the stair's cull of it agrees, and
the mesh, the collision list and the selection boxes of a scene agree on which
arms exist.

Scenes also cross a Chunk boundary through `Neighborhood::new` with a real
neighbour, since the shell is where an arm toward the next Chunk is read.

## Performance

Baselines are saved with criterion before the first code change and compared
after: `cargo bench -p tesserae-core -- --save-baseline main` on the commit
this branches from, `--baseline main` after. Apple M5, release.

- **Pass 1 and the light solvers must not move.** `greedy_mesh/terrain`,
  `/solid`, `/checkerboard`, `light_chunk/straddling_settled`, `/flooded_air`,
  `mesh_chunk/straddling`. Step 5 found a 2-6% regression from an innocuous-
  looking `seals` call in the pass-1 inner loop, so "within noise" is the bar
  and a measured regression is explained or reworked, not accepted.
- **Terrain pays nothing for connected shapes.** `terrain_stairs_at_cap` and
  `all_stairs` must not move either, since pass 2 gains a branch.
- **The sweep and ray have no benchmark at all today.** Ticket 01 adds them on
  the unchanged code and saves them in the baseline, so decisions 7-9 have a
  before. Cases: a fall onto a floor of cubes, a walk along a wall, a ray across
  terrain, and the same three on a floor of slabs, so a regression in the
  common non-connected path shows.
- **Connected shapes get their own numbers**: a Chunk with a fence line, a Chunk
  of fences at the cap (the number decision 10 turns on), and registration time
  for a fence and a six-way pipe.

The PR reports before and after for each, and the streaming worst tick is
re-derived from them as in step 5.

## Known loose ends, declared rather than discovered

- **Groups are per Shape, not per Material** (decision 2). Step 7's registry
  replaces this.
- **Overhang is at most one Voxel.** A two-Voxel-high gate needs a multi-Voxel
  structure; this is a bound on a cheap mechanism, not on the design.
- **Connected Shapes cannot be rotated** (decision 4).
- **LOD under-counts a chunky connected Shape** (decision 5).
- **Terrain places no fences**, as no shaped Voxel; see step 5's loose end.
- **A fence still cannot be jumped by a controller that does not exist.** The
  geometry is there; the policy is the controller's.
- **A fence gate** (a connected Shape whose arms move) is a Voxel with state
  that changes without an Edit, and is beyond a table row.

## Questions for review

None of these blocks the work, and each has a default the tickets implement;
change the default now and the spec changes with it.

1. **Derived, not stored** (decision 1). The one with a real migration cost if
   wrong later, which is why it has an ADR.
2. **Groups on Shapes** until Materials exist (decision 2).
3. **Connected Shapes are unoriented** (decision 4). A wall-mounted connecting
   Shape would need this relaxed, at 24 times the registration work.
4. **Picking switches to the drawn shape** (decision 9) - the only behaviour
   change to something that already works.
5. **The cap**: weighted only if the benchmark says so (decision 10).

## Done when

`./scripts/cargo-ci` passes in full; the hole-free test passes over scenes with
connected Shapes across all of its seeds; the baseline comparison shows pass 1,
the light solvers and terrain meshing unmoved, with the numbers in the PR; a
`dev` screenshot shows a fence line growing arms and a corner, and the sweep
test stands a body on the top of one at 1.5 Voxels; ADR-0010 is written and
ADR-0006's Consequences points at it; and `CONTEXT.md` and DESIGN.md's step 5
loose ends are updated.
