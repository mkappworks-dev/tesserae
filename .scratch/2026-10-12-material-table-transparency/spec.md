# Material table and transparency (glass, leaves, water)

Off the build order, like caves: it sits between step 5 (shaped Voxels) and step
7 (Mods), and exists because step 5 left a declared gap. A full cube always
hides its neighbours' faces and always stops light, and `ShapeTable` answers
geometry only. Glass, leaves and water need three answers a Shape cannot give:

1. Does this Material hide the faces of the Voxels around it? (Glass beside
   glass: yes. Leaves beside leaves: no. Glass beside stone: stone's face must
   stay.)
2. How much does light lose passing through it? (Glass: nothing. Leaves: a
   little. Water: more. Stone: all of it.)
3. Does it need a separate, blended render pass, with the sorting that implies?

Step 5's decision 5 already put the second answer in the right place: the
reduction "is a property of what a Voxel is made of, not of its geometry, so it
belongs with Materials". This spec is that table, and the first consumer of it
besides `emission`, which is a `match` on a placeholder id today.

## Vocabulary this adds

To `CONTEXT.md` in the first ticket.

- **Dimming** - the levels of light a Voxel removes from light entering it, on
  top of the one level every step already costs. 0 for air and glass, 15 for
  stone. Dimensionless, like every light quantity (ADR-0003).
- **Opaque** - a Material that hides every face of its neighbours and dims light
  by 15. Stone.
- **Translucent** - any Material that is not Opaque. Glass, leaves, water.

## Decisions

### 1. A Material is a row in a table, indexed by `MaterialId`

The same call ADR-0006 made for Shapes, for the same reason: a Mod declares
Materials, a Rust `match` cannot grow at load time, and the id-to-definition
mapping becomes save data. So a `MaterialTable` in a new
`tesserae-core/src/material.rs`:

```rust
pub struct MaterialDef {
    pub emission: u8,       // block light the Voxel emits, 0..=15
    pub dimming: u8,        // 0..=15, see above
    pub occlusion: Occlusion,
    pub render: RenderClass,
}
pub enum Occlusion { Opaque, SameMaterial, Never }
pub enum RenderClass { Opaque, Blend }

impl MaterialTable {
    pub fn builtin() -> Self;
    pub fn register(&mut self, def: MaterialDef) -> Result<MaterialId, MaterialError>;
}
```

`register` is called once per declared Material in a batch at load, never per
Voxel, so ADR-0001 holds by construction: the WASM side hands over a list of
`MaterialDef`s and everything after is a native array lookup. `emission` and the
new properties share the one table, which is the sharing the brief asks for.

Storage is a dense `Vec` of four-byte rows, **sized to the registered count**.
The `emission` comment rejected a table indexed by `u16` because that is 64 KiB
of zeroes; ids are handed out sequentially, so a table of N Materials is 4N
bytes and a lookup is one bounds-checked load.

Built-ins, with ids that are **permanent** (they are save data, as Shape ids 0
to 3 are): 0 air (reserved; air is decided by `Voxel::is_air` without the
table), 1 stone, 2 dirt, 3 grass, 4 lantern, then three placeholders the
transparency work needs:

| id  | name   | emission | dimming | occlusion      | render |
| --- | ------ | -------- | ------- | -------------- | ------ |
| 1-3 | terrain | 0       | 15      | `Opaque`       | Opaque |
| 4   | lantern | 15      | 15      | `Opaque`       | Opaque |
| 5   | glass   | 0       | 0       | `SameMaterial` | Blend  |
| 6   | leaves  | 0       | 1       | `Never`        | Opaque |
| 7   | water   | 0       | 2       | `SameMaterial` | Blend  |

Water is here only as a Material. Its flow is step 8 and its shading step 9;
reserving the id now means neither renumbers anything.

**An id the table does not know is Opaque, emits nothing, and is drawn.** This
is the opposite choice to an unknown Shape, which is invisible, and deliberate:
a Shape's geometry cannot be guessed, but a Material's worst case is a solid
stand-in cube in the missing-Material colour, which keeps the World walkable and
tells the player something is wrong. Invisible-and-intangible would leave holes
in the ground. The same stand-in is what keeps a pre-table save loading.

**Validation** at `register`, so no consumer re-checks: `emission` and
`dimming` at most `MAX_LIGHT`; `Occlusion::Opaque` requires `dimming == 15` and
`RenderClass::Opaque` (an opaque Material that lets light through is a light
leak by construction, and a blended Material that hides everything behind it is
a draw call spent on nothing); at most `u16::MAX` rows.

Appearance is **not** in the table: colour, alpha amount and texture are Look
pack data (`CONTEXT.md`). `RenderClass` is the one rendering fact that is
semantic, because it decides which mesh a face goes into and so is needed by
the mesher, which knows nothing of Bevy.

Not in the table yet, declared rather than discovered: whether a body collides
with it (water is not solid to a sweep; `collide` takes no Material today), a
name, and a hardness. Each is a row field when something reads it.

### 2. Dimming is the destination's, and opacity is the case where it saturates

The light rule becomes `transmitted(channel, level, face)` minus the **entered**
Voxel's dimming, saturating at zero. Nothing about `transmitted` changes, in
particular the exception that makes full-strength sky fall straight down
undimmed. So sky 15 falls into glass at 15 and into water at 13, and entering
the same Voxels sideways it is 14 and 12. A dimming of 15 is the opaque case, and for it
geometry decides instead and nothing is subtracted: a stone stair stores the
light of its own notch undimmed, as it did in step 5, and blocks the faces it
seals. Dimming below 15 is the reverse - every face open, the dimming subtracted
once per entry. `Tables::light_params` is that one rule, as `transmitted` is the
one rule for the step.

Both channels use the one value: water dims a lantern's glow as it dims the
sun. One number is also what Minecraft authors, and nothing here argues for two.

**Geometry stays Shape's, but only for Opaque light.** A Voxel blocks light by
its sealed faces (`ShapeTable::light_mask`) only when its Material's dimming is
15. Below that, light passes every face and the Material's dimming applies
once per entry, whatever the Shape. So a glass stair does not cast the stair's
shadow (correct: it is glass), and a half-height water slab dims by the same 2
as a full one. This is an approximation, chosen for the same reason step 5 chose
sealing over authoring: the alternative needs a fill fraction per Shape per
face and nothing visible argues for it.

The Voxel's own cell holds the light that reached it, as air does. This is the
change that matters downstream: a translucent cube now stores a non-zero light,
where "an opaque Voxel holds 0 by construction" was what made the mesher's
"light of the Voxel in front" rule work. It still works, and now lights a stone
face seen through water by the water's own, depth-attenuated, level.

### 3. Occlusion: a face is hidden by what proves it hidden

For a full-cube Voxel `here` and the Voxel `across` a face, the face is hidden
iff `across` hides it:

- `across` is a full cube and `Occlusion::Opaque`; or
- `across` is a full cube, `here` and `across` are the same Material, and that
  Material is `Occlusion::SameMaterial`; or
- (shaped) `across`'s coverage contains the face's rectangle **and** one of the
  above holds for its Material.

Leaves (`Never`) hide nothing, not even other leaves, and so draw every face as
Minecraft's "fancy" leaves do. Glass hides glass, so a wall of glass is a skin
and not a honeycomb. `SameMaterial` compares Material ids and not "both
translucent", so two different glasses keep the face between them: a stained
pane beside a clear one shows its edge.

The failure mode is the step-5 one and stated the same way: an unproven face is
drawn, never dropped, so the error is overdraw and not a hole. The brute-force
scene test from step 5 is extended with translucent Materials and a ground
truth that applies the rule above on the 1/16 grid.

### 4. The mesher splits on `RenderClass` and the merge key does not change

Greedy pass 1 merges on `(MaterialId, Light)`. Transparency does not touch it:

- A merged quad is one Material, and a Material has one `RenderClass`, so no
  quad ever straddles the two passes. Routing is by `render(material)` at
  emission, off the same mask, in the same loops.
- Culling (decision 3) changes which cells are in the mask, never what two cells
  must agree on, so it can only split a run that would have merged, and a
  `SameMaterial` run of glass that is hidden on one side and not the other
  splits where the interior face begins. That is correct and is the bulk of the
  cost glass adds.
- Light is already in the key because a quad has four vertices. A translucent
  Voxel in front of a face now has a light of its own, so a stone floor under
  water splits into one run per depth. Fifteen levels bound it, and it is the
  same cost a lantern's gradient already pays.

`ChunkMesh` becomes two layers:

```rust
pub struct MeshLayer { positions, normals, uvs, materials, light, indices }
pub struct ChunkMesh { pub opaque: MeshLayer, pub blended: MeshLayer, pub shaped_voxels: u32 }
```

Pass 2 routes each shaped Voxel's quads by its Material the same way. The
client builds one `Mesh` per non-empty layer (decision 6); until ticket 04 it
builds the opaque one only, so a blended layer is meshed and not drawn.

### 5. The hot paths pay nothing for terrain, by specialisation

Two things in this crate are measured and fragile, and both are constraints on
the design:

- `light::flood` is specialised on `<SHAPED, CHANNEL>` because its speed was
  found to depend on the optimiser seeing both as constants.
- Pass 1 tests `is_full_cube()` first and alone because `seals` for every face
  measured 2-6% slower on `solid` and `terrain`.

So translucency is a third const generic and a **per-Chunk gate**, in the style
of `may_contain_shaped`:

- A Chunk is **translucent** if its palette holds a non-air Voxel whose Material
  is not Opaque. A palette scan, tens of entries; terrain pays it once per job
  and never takes the branch.
- `flood::<SHAPED, CHANNEL, DIMMED>`: with `DIMMED = false` the loop is the one
  that ships today, instruction for instruction. Only a translucent Chunk takes
  the `DIMMED = true` instantiation, which reads a parallel `dim` array and
  subtracts after `transmitted`. Light needs the centre Chunk only: dimming is
  the entered Voxel's, and what arrives from a neighbour already includes the
  neighbour's own.
- Pass 1 becomes `pass1::<TRANSLUCENT>`. Meshing needs the centre **and** the
  six neighbours, because a glass neighbour fails to hide a stone face.
  `Neighborhood::new` therefore records the Voxels of the seven palettes (a copy
  of tens of entries, against a 39 thousand Voxel copy it already does) so
  `greedy_mesh` decides the gate without a scan of the padded array.
- The gate is evaluated **after** `mesh_chunk`'s `emits_no_faces` and
  `light_chunk`'s uniform shortcuts, which answer in nanoseconds and must stay
  so.

The shortcuts themselves change meaning, not cost. "A uniform full cube emits no
faces if every neighbour is one" becomes "...an Opaque (or the same
`SameMaterial`) cube"; "a uniform opaque Chunk lights dark" becomes "a uniform
Chunk of dimming 15 and no emission"; each is one table lookup on a path that
already runs only for uniform Chunks.

The sweep that fills `blocked` and finds emitters in `light_chunk` reads
every Voxel of a non-uniform Chunk, and replacing a `match` on one id with a
table load there is the single change in this spec that sits on a per-Voxel
path for terrain. It memoises on "same Voxel as the last", which terrain's runs
make nearly every read, and is measured before it is believed.

### 6. The render pass: a second entity per Chunk, blended, sorted by Bevy

Bevy 0.20's `AlphaMode::Blend` is the base `StandardMaterial`'s, so the chunk
material keeps its shape and a second `ChunkMaterial` with `alpha_mode: Blend`
draws the `blended` layer. The palette's `Vec4` already has an unused `w`, which
becomes alpha: the amount is Look pack data, the class is Material data
(decision 1).

Sorting is Bevy's: transparent items sort back to front **per entity**, by the
entity's position. That is chunk-granular, correct between Chunks laid out on
a grid and wrong inside one. Inside a Chunk the order is the mesher's emission
order, so two overlapping translucent surfaces (a glass house with water behind
it) can blend in the wrong order. A single water surface or a single pane, which
is nearly everything, is unaffected. Options for when it is not, in cost order
and none built here: sort a Chunk's blended quads on camera movement (what
Minecraft does); weighted-blended order-independent transparency, which Bevy 0.20
ships as an experimental plugin; or accept it. The ticket measures the artefact
on a screenshot before choosing.

Leaves render in the opaque pass. They are cutout in every voxel game, which
needs an alpha test against a texture, and textures are step 9; until then a
leaf is a dimming Material that draws as a solid cube. `RenderClass` gets a
`Cutout` when that arrives and not before.

Forward rendering is what this assumes and is what DESIGN.md already says a
deferred path would need anyway ("still needs a forward transparency pass").
Blended meshes sit outside SSAO and TAA's inputs in Bevy; whether they cast
shadows is read from the 0.20 source in the ticket and not assumed here.

### 7. The Mod boundary

One batched `register` per Material at load, the same shape as `ShapeTable`. A
Mod's Materials take ids from 8 in registration order, so a World's save needs
the name-to-id registry that ADR-0006 already says Shapes need; it is one
registry for both, which is step 7's. `Tables { shapes, materials }` is the
single value the loader fills and the Server holds, so a native consumer takes
one reference and no call site learns that there are two tables.

### 8. Consequence for LOD (step 6, decided here)

The mip chain aggregates by dominant Material (DESIGN.md). With translucency a
coarse Voxel's properties must come from one row, not a blend, so: the dominant
Material is chosen by Shape volume as for solidity, a coarse Voxel never mixes
`RenderClass`, and a half-water half-air column is water or air whole. Step 6
writes it; this records that the table, not the Voxel, is where it looks.

## Alternatives rejected

- **A translucency bit in the Voxel** (the spare orientation bits). Material
  properties would be duplicated into every palette entry, a Mod changing a
  Material's dimming would invalidate saved Voxels, and `is_full_cube` would
  stop meaning one thing. The Voxel stays `(Material, Shape, Orientation)`.
- **One `transparent: bool`.** Cannot say glass hides glass and leaves do not,
  or that water dims and glass does not. Two of the three required answers
  collapse.
- **Dimming on `ShapeDef`.** Step 5's decision 5, and the reason a stair of
  glass and a stair of stone differ.
- **A dense `[T; 65536]` table.** 64 KiB of zeroes, the thing `emission`
  avoided.
- **Always asking the table in pass 1 and the flood.** Simpler, and what the
  benches in this repo exist to rule out; decision 5.
- **Per-face dimming from the Shape's fill fraction.** Right, expensive, and
  invisible next to the approximation in decision 2.

## Invariants, and where each is enforced

1. **`tesserae-core` gains no dependency.** `material.rs` is plain data and
   arithmetic; render class is an enum core owns and the app maps.
2. **No Mod reaches core.** One batched `register`; reads are native.
3. **Everything physical is in metres.** Nothing here is physical: dimming is a
   count of light steps and does not scale with Voxel size, and the table holds
   no distance.
4. **The Server is the sole authority.** The table is read-only to the Client,
   which receives it as it receives `Shapes` today.

## Performance

Baseline: `cargo bench -p tesserae-core --bench meshing -- --save-baseline
before`, saved on this branch before any code, Apple M5, release. Every ticket
compares against it with `--baseline before` and the PR reports the table.

The numbers to watch, in the order they are likely to move:

- `light_chunk/straddling`, `/straddling_shaped`, `/cave_*` - the sweep's
  per-Voxel emission lookup (decision 5).
- `mesh_chunk/declined_enclosed` - the uniform-cube shortcut gains a lookup.
- `greedy_mesh/terrain`, `/solid`, `/checkerboard` and `light_chunk/flooded_air`
  - must not move: the specialisation is meant to leave them byte-identical.
- New, because nothing measures translucency yet: `greedy_mesh/terrain_water`,
  `/glass_room`, `light_chunk/straddling_water`, `/leaves_canopy`.

A regression is reported with its size and cause or the change is reworked, as
in step 5. The streaming worst tick is re-derived if a stage's cost moves.

## Tickets

01, 02 and 03 landed together in the first PR, at the author's request; 04 and
05 are separate.

| #   | Ticket                                                        | Blocked by |
| --- | ------------------------------------------------------------- | ---------- |
| 01  | `MaterialTable`, `Tables`, `emission` off the `match`, ADR    | -          |
| 02  | Light dimming in both solvers, behind the translucent gate    | 01         |
| 03  | Occlusion by Material and the two-layer `ChunkMesh`           | 01         |
| 04  | Blended pass in the app, hotbar, screenshot                   | 03         |
| 05  | The Server's edit path rejects unregistered Materials         | 01         |

02 and 03 are independent of each other: a translucent Voxel with dimming but
no culling rule lights correctly and draws wrongly, and the reverse, and both
are testable alone. 04 is the first time a player can see any of it.

## Known loose ends, declared rather than discovered

- **A partial emitter can be erased by the decrease pass.** `relight`'s
  `darken` clears any Voxel at or below what the removed source could have
  delivered, which is safe while every emitter is 15 (nothing is brighter than
  the lantern beside it). With a table that can declare emission 8, an emitter
  at 8 beside a removed 15 holds 14 and is cleared, and nothing re-seeds it.
  Latent in 01 (the only emitter is 15), and ticket 02 fixes it with a test
  before a level-8 Material can be registered by anything but a test.
- **Joint coverage of the mesh.** Two partial Shapes that jointly cover a face now
  block light (the joint-coverage change that landed on `main`), and
  `Tables::joint_id` leaves clear Materials out of it, so two glass halves seal
  nothing. The mesher still tests each quad against one neighbour's coverage, so
  a face two neighbours jointly hide is drawn: overdraw, never a hole.
- **No intra-chunk sorting** of blended quads (decision 6).
- **Collision and picking** treat water as solid: `collide` reads Shapes only.
  A `solid` row field when a controller exists.
- **Cutout leaves** need textures (step 9).
- **Worldgen places none.** Terrain contains no translucent Voxel, so a player
  sees them only by placing them, as with shaped Voxels in step 5.

## Done when

Per ticket: its acceptance criteria and `./scripts/cargo-ci`. For the feature as
a whole: a screenshot from the `dev` capture of glass, leaves and water, lit and
blended; `cargo bench -p tesserae-core --bench meshing -- --baseline before`
shows no unexplained regression; and the streaming worst tick is re-measured
against the 15.6 ms a tick has at 64 Hz.
