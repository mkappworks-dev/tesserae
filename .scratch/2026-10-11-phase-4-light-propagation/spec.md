# Light propagation (incremental flood-fill)

Build-order step 4. Shadow maps handle the sun; thousands of point lights for
torches is not viable on any GPU, so light is a per-Voxel flood-fill over the
0-15 model, baked into the mesh as a vertex attribute. It must be incremental:
one torch re-floods a small region, not a Chunk.

## Vocabulary this step adds

Three terms, to be added to `CONTEXT.md` in the same PR.

- **Light level** - how much light reaches a Voxel, 0 to 15. Dimensionless, and
  deliberately so: it is a count of attenuation steps, not a distance, so it
  does not scale with Voxel size. ADR-0003 governs anything physical, and a
  light level is not.
- **Sky light** - the light a Voxel receives from the sky. Full strength falls
  straight down undimmed, so open ground is uniformly lit however tall the air
  column above it is.
- **Block light** - the light a Voxel receives from emitting Materials. Dims by
  one level per Voxel in every direction, the -Y direction included.

Two channels rather than one because they are shaded differently: sky light is
how much of the sky dome and sun a face can see, and block light is a local
source with its own colour. Collapsing them loses the ability to make night
dark without also making caves bright.

## Decisions

### 1. Light lives in a parallel per-Chunk array, not in the palette

`ChunkLight` is a new type in `tesserae-core/src/light.rs`, held by the Server
alongside the `Chunk` and keyed by the same `ChunkCoord`. `Chunk` and `Voxel`
are untouched.

Putting light in the palette was considered and is wrong twice over. A `Voxel`
is `(Material, Shape, Orientation)` by definition in `CONTEXT.md`, and light is
not part of what a Voxel *is* - it is a property of the position. Worse, it
would destroy palette compression: a uniform stone Chunk with a light gradient
across it has one palette entry today and would need one per distinct
(Material, light) pair, so the Chunk that compresses best is exactly the one
that would stop compressing.

`Light` packs both channels into one byte, `sky:4 | block:4`, mirroring the
0-15 model. `ChunkLight` mirrors `Chunk`'s storage shape - `Uniform(Light)`
promoting to a dense `Box<[Light; CHUNK_VOLUME]>` on the first differing write.
The uniform case is not a micro-optimisation: a streamed 3D ring is mostly open
sky (uniform sky 15) and mostly deep rock (uniform 0), and only the Chunks the
surface crosses go dense. At 32 KiB per dense Chunk, a 2167-Chunk fill with
roughly a tenth of them dense costs about 7 MB.

### 2. A boundary is crossed by a boundary condition, as geometry's is

`mesh_chunk` already solves this problem for geometry with its
`[Option<&Chunk>; 6]` gate. Light reuses the shape: the bulk solver takes the
Chunk and the light of its six face neighbours, and nothing else.

```rust
pub fn light_chunk(
    center: &Chunk,
    border: [Option<&ChunkLight>; 6],
) -> ChunkLight
```

Neighbour *geometry* is deliberately absent. An opaque Voxel stores light 0, so
"neighbour's light, dimmed by one" is already blocked by an opaque neighbour -
the opacity is encoded in the light the neighbour carries. A `None` face is a
dark boundary, not an air boundary.

The Server runs this to a fixed point by relaxation: a Chunk whose light changed
marks its six neighbours light-dirty, and they re-solve. Light levels are
bounded and a fresh Chunk starts dark, so every change is an increase and the
relaxation converges.

**The open-sky boundary is the one case the relaxation cannot discover.** Sky
light descends undimmed, so the top of the loaded volume has no neighbour to ask
and getting it wrong darkens every Voxel in the column below - an error that
propagates unbounded, unlike a lateral one which cannot reach further than 15
Voxels and so cannot leave the load extent. The Server supplies it directly from
the generator: for a Chunk whose +Y neighbour is not loaded, the +Y boundary is
a synthetic uniform sky-15 `ChunkLight` when that neighbour's floor is above
`Worldgen::surface_bounds_m().1`, and dark otherwise. Above the band the
generator produces air everywhere, so the test is purely vertical and exact. The
synthetic `ChunkLight` is the `Uniform` variant, so it costs no allocation.

Relight work is walked **top-down** - the load extent's offsets sorted by
descending y, then by distance - rather than nearest-first like generation. Sky
light flows downwards, so solving a Chunk before the one above it means solving
it again; the order is what keeps the common case to one pass.

### 3. There is no light-only mesh path, and that is forced

Light joins the greedy mesher's merge key. A merged quad has four vertices and
cannot carry a different light level per Voxel it spans, so two coplanar faces
merge only when their Material *and* their light agree.

That makes the question "what does re-meshing cost when only light changed?"
answer itself: a light change can change the quad decomposition, so the quads
cannot be reused and a light-only path would be unsound. A light change marks
the Chunk mesh-dirty and it re-meshes in full, at the `mesh_chunk/straddling`
cost.

The cost of the merge key is small where it matters and bounded where it does
not. Open terrain is uniformly sky 15, so nothing that merged before stops
merging; a lantern shreds merging only within the 15 Voxels it reaches.

A face is lit by the light of the Voxel **in front of** it, not the light inside
the opaque Voxel it belongs to - which is 0 by construction, and is what makes
the dimming rule work in the first place. For a face on a Chunk boundary that
Voxel is in the neighbour, so `Neighborhood` grows a parallel light shell beside
its Voxel shell. To keep the argument count honest, `mesh_chunk` and
`Neighborhood::new` take a `LitChunk<'a> { chunk, light }` pair rather than two
more arrays.

### 4. Both `mesh_chunk` fast paths survive, for a reason worth stating

Light can only split a quad that would otherwise merge. It never creates or
removes a face, because face visibility is a question about geometry alone - a
full cube next to a non-full-cube. So:

- **Uniform air centre** emits no faces, so no vertices, so no light attribute.
  Unchanged. But a uniform-air Chunk still needs its *light* computed, because
  its neighbours read it across the boundary: the light solver must not inherit
  the mesher's fast path, and it does not, being a separate function.
- **Uniform full cube enclosed by six uniform full cubes** emits no faces
  whatever the light is, because every face is occluded from the other side.
  Unchanged. A test asserts it against a Chunk given deliberately non-uniform
  light, so the claim is checked rather than argued.

### 5. The vertex attribute, and what the shader does with it

`ATTRIBUTE_LIGHT`, `VertexFormat::Uint32`, shader location 11 beside
`ATTRIBUTE_MATERIAL` at 10, declared in `ChunkMaterialExtension::specialize`'s
spelled-out vertex layout. One `u32` carrying `Light`'s packed byte. Packing it
into `ATTRIBUTE_MATERIAL`'s spare 16 bits would save 4 bytes a vertex, but those
bits are already reserved for Shape in step 5 and the saving is noise against
the 32 bytes of position, normal and UV.

In WESL, read from `bevy_pbr`'s own `PbrInput` rather than multiplied into
`base_color` the way Minecraft does it, so albedo stays albedo and the BRDF
keeps working:

| Channel     | `PbrInput` field                              |
| ----------- | --------------------------------------------- |
| Sky light   | `diffuse_occlusion` and `directional_shadow_factor` |
| Block light | `material.emissive`                           |

`diffuse_occlusion` multiplies the ambient dome in `pbr_ambient.wesl`, and
`directional_shadow_factor`'s own doc in `pbr_types.wesl` says "use this for
custom shadow maps" - which is what a sky-light flood-fill is. Driving both is
what makes a roofed space dark: the shadow map alone stops working past the
last shadow cascade, and the ambient dome alone would leave the sun leaking in.

`lightmap_light` is semantically the closer fit for block light but needs the
`LIGHTMAP` shader def and a second UV set, so block light goes to `emissive`,
tinted by the Voxel's own palette colour so a lit face still shows its Material.

`spawn_light`'s `ambient.brightness = 900.0` stops being a flat fill: the
`GlobalAmbientLight` becomes the sky dome's radiance and the flood-fill's sky
channel gates it per face. On today's heightmap terrain every visible face has
an open column above it, so sky light is uniformly 15 and the picture is
unchanged - which is the honest outcome, not a bug. See "Known loose ends".

### 6. Two solvers, because one would be either wrong or too slow

- **Bulk** - a Chunk arrives. Solve all 32768 Voxels at once over contiguous
  arrays, from the six boundary planes, the Chunk's own emitters and the sky
  seed. This is the per-tick cost.
- **Incremental** - a Voxel is edited. A voxel-granular BFS that touches only
  the region within 15 Voxels of the edit. This is what DESIGN.md's "one torch
  re-floods a small region, not a Chunk" asks for; a per-Chunk re-solve is
  literally the thing it rules out.

Seeding the bulk case through the incremental path was considered: a new Chunk's
boundary is 6144 seed Voxels, and pushing them through a random-access interface
costs orders of magnitude more than one contiguous sweep. Running the
incremental case through the bulk path is worse than slow, it is **wrong** -
a from-scratch per-Chunk re-solve is a Gauss-Seidel iteration over a max
system, which converges upwards but not downwards. Two Chunks each holding
light up at the other's level is a stable fixed point with no real source
behind it, so removing a light would leave it lit forever. The incremental
decrease pass is sound because it tracks provenance: clear what this source
lit, then re-propagate from the independent sources it uncovered.

The invariant that keeps them from fighting: **an edit uses the incremental path
only, a Chunk arrival uses the bulk path only.** Both share one `transmitted()`
rule and one opacity test, and a test drives the same scenario through each and
asserts they agree.

The incremental solver reads and writes through a `LightVolume` trait the Server
implements over its two maps, so `tesserae-core` never learns how Chunks are
held and acquires no dependency - it still has none at all. Coordinates are
`[i32; 3]`, not `IVec3`, for the same reason.

### 7. Where the Server puts it

`ChunkStore` gains a second map of `ChunkLight` with `pub(in crate::server)`
mutators, exactly like `chunks`. Light is World state and the Server is its sole
authority; the Client receives it already baked into the vertex attribute and
gains no read access. `streaming::stream` gains a `relight` stage between
`generate` and `publish`, with its own entry in `StreamingBudget`.

`generate`'s standing comment that "a neighbour arriving can never invalidate a
mesh the Client already holds" **stops being true in this step**, and not for
the reason it predicted. Light crosses Chunk boundaries, so a Chunk two Cells
away arriving changes the light in between and so changes ours. The comment and
`no_mesh_the_client_still_holds_is_stale` both have to move: a relight that
changes a Chunk's light marks it mesh-dirty, and the Client's existing
replace-in-place upload path already handles the redelivery.

## Known loose ends, declared rather than discovered

- **Sky light is uniform on the terrain that exists.** `worldgen` is a
  heightmap: no caves, no overhangs, so every visible face has an open column
  above it and sky light is 15 everywhere. The flood-fill is therefore only
  *visible* where something encloses space, which is why this step ships a
  `dev`-only structure (ticket 07) rather than a worldgen change. Caves belong
  to worldgen, not to light.
- **Block light has no emitter in generation.** A `LANTERN` placeholder Material
  joins `STONE`/`DIRT`/`GRASS`, and `worldgen` places none of it. The edit API
  that a Mod or a player would use to place one is build-order step 5; this step
  provides the solver it will call.
- **Per-shape opacity is step 5.** Light opacity follows `Voxel::is_full_cube`,
  which treats a shaped Voxel as transparent - the same conservative choice the
  mesher already makes, and the same line in DESIGN.md's "Shaped voxels" list.

## Done when

`cargo dev` shows flood-filled lighting with the flat ambient fill gone,
`./scripts/cargo-ci` passes in full, `cargo bench -p tesserae-core` shows no
unexplained regression against the step-3 baselines, and the streaming worst
tick is re-measured against the 15.6 ms a tick has at 64 Hz.
