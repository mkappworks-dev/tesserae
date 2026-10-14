# Light the coarse LOD levels: a covered-column rule

Branch `feat/lod-coarse-light`. One ticket,
[01](issues/01-light-coarse-lod-levels.md), which holds the problem, the
Step 0 capture and the comparison of options; this spec is option (a) of it,
built. Read the ticket's Comments first: Step 0 leaned `wontfix` and the
maintainer chose to build anyway, so the expected gain is a few dozen pixels per
mouth at 250 m and the work is kept as small as the rule allows.

## What changes

A Chunk above level 0 is still meshed with no stored light. What changes is the
light the mesh job hands the mesher. Today that is `Light::new(MAX_LIGHT, 0)`
for the Chunk and its six neighbours. After this step it is full sky except for
**covered** air Voxels, which get `COVERED_SKY`.

**Covered.** A coarse Voxel is covered if it is air and its top lies more than
one coarse Voxel below the terrain surface over it. The surface is
`Worldgen::surface_height_m` of the level's generator, in metres, read at the
four corners of the Voxel's footprint and taken as the lowest of them. In
metres throughout: the Voxel's size is `voxel_size * 2^level`, never a shift of
an index, so a World with smaller Voxels gets the same covered set.

**What a covered Voxel does.** The mesher lights a face by the light of the
Voxel across it, so a face looking into a covered Voxel is lit `COVERED_SKY`
and every other coarse face is lit as it is now. Nothing is stored: the light
is built in the mesh job from the generator and the Chunks the job already
holds, and dropped with it. `Light::new(0, 0)` is the constant's value: level 0
shows the inside of a cave black (Step 0, the 45 m capture), and a covered Voxel
is by construction at least one Voxel, 2 m or more, under open sky.

**Why a Voxel of margin.** The downsample rounds the surface up: the top solid
Voxel of a column is within one coarse Voxel above `surface_height_m` and no
more than half a Voxel below it (ticket 08 of the LOD spec pins it). The air
above the top Voxel is therefore never more than half a Voxel under the surface,
so a margin of one Voxel leaves every open-ground face at sky 15, byte for byte.
The lowest of four corners and not the lower corner alone, because on a slope the
corner can sit a Voxel above the middle at level 5. If a sweep shows the margin
is not enough for the cave-free case, the fix is more sample points, not a wider
margin: a wider margin darkens real mouths.

**Where it lives.** `tesserae-core/src/lod.rs`, beside the downsample, so the
rule is headless and benchmarkable and the server has one call to make:

- `COVERED_SKY`, a `Light`, and a `Surface` trait (a height in metres over a
  point and the band it is confined to) that `Worldgen` implements, so a test can
  stub the heightfield and count the samples.
- `coarse_light(surface, voxel_m, coord, center, around)` returning a
  `CoarseLight` of a `ChunkLight` for the Chunk and one for each neighbour, in
  the mesher's face order. The mesher reads a neighbour's light only on the
  plane it shares, so a neighbour's light needs only that plane to be right.
- The Chunk's corner heights are sampled once as a 36 by 36 lattice that
  covers the Chunk and its one-Voxel shell, and every Voxel asks the lattice.
  The six neighbours do not sample again: above and below share the Chunk's
  columns, and the four side neighbours' shared plane is inside the shell.

**Fast paths**, so the cost lands on the Chunks the surface band crosses and
nowhere else:

- Nothing in the seven Chunks can be covered when the lowest Voxel top of the
  padded range is not under `surface_bounds_m().1` less one Voxel: every light
  is the shared open-sky static, no allocation, no sample. This is nearly the
  whole upper ring.
- A uniform Chunk of solid Material has no air to light: open sky, whatever
  its neighbours are.
- A uniform air Chunk wholly under `surface_bounds_m().0` less one Voxel is
  covered throughout: a filled `COVERED_SKY`, no sample.
- Anything else samples the lattice and sets the covered Voxels, which promotes
  to a dense array only for the Chunk or neighbour that has one.

**The mesh job.** `mesh_coarse` takes the level's generator (an `Arc` or a
clone, as the generate job already does, so ADR-0007 holds: a job holds owned
values and returns an owned `ChunkMesh`) and the level's Voxel size in metres.
Stages stay two. `mesh.rs`, `light.rs` and `ChunkLight` are unchanged. A World
without caves meshes exactly as before.

## What it gets wrong, accepted

- **A pit open to the sky** is dark below its top Voxel here and lit at level
  0, where sky light falls straight down undimmed. The surface height is the
  heightfield's, before caves. A column scan of the Chunks above would fix it
  and needs Chunks the job does not hold.
- **A sideways opening** is dark from its first covered Voxel, where level 0
  grades over about 15 m. A few Voxels of lip at 2 to 4 m.
- **`COVERED_SKY` is a constant,** not a function of depth.
- **Sealed tunnels are covered too,** which costs nothing visible and is what
  makes the rule a column rule and not a flood.
- **Translucent coarse Voxels** (glass, water, leaves) are not lit by the rule,
  which asks for air. The generator makes none above level 0 and no Edit reaches
  one.

## Invariants, and where each is enforced

1. **The Server is the sole authority over World state.** The change is in a
   job that returns a mesh; no Client file is touched.
2. **No Mod reaches `tesserae-core` directly.** No host call is added.
3. **`tesserae-core` gains no dependency.** It already holds `Worldgen`.
4. **Everything physical is in metres.** The margin is a Voxel of the level
   and every height is `surface_height_m`'s.
5. **A job holds Arcs and returns an owned value.** No `LodStore`,
   `ChunkStore` or `LodState` in `mesh_coarse`'s signature.

## Tests

In `tesserae-core` unless noted; each is written first, watched to fail, and
mutation-checked.

- **A hand-built tunnel under a flat heightfield** (a stub surface and a
  Chunk with an air tube): the faces inside get `COVERED_SKY`, the faces above
  the surface get 15, the Voxel exactly one Voxel under the surface stays lit and
  the next one is covered. Mutations: a margin of two Voxels, a margin of half
  a Voxel, the lattice read at one corner.
- **The Chunk border.** A tunnel crossing the Chunk's border on each of six
  faces: the Chunk and its neighbour agree on the covered state of the plane
  they share. Mutation: a neighbour's plane computed from the neighbour's own
  columns only.
- **Fast paths.** A Chunk wholly above the band returns the open-sky light
  and takes no surface sample (counted through the stub); a uniform solid
  Chunk does the same; a uniform air Chunk wholly under the band is covered
  throughout, with no sample.
- **The accepted pit.** A pit open to the sky is covered below its top Voxel;
  the test is named for the limitation, so a later fix flips it on purpose.

In `crates/tesserae/src/server/lod.rs`:

- **Cave-free identity.** With `caves: None`, over a sweep of columns at
  levels 1 to 5 across the 4 km square, every coarse Chunk the sweep meshes
  has the same mesh as with the open-sky light: no Voxel is covered where there
  is no cave. Mutations: a margin of zero, the lower corner only.
- **A real mouth.** The default seed's mouth at (-26, -352) m, level 1, Chunk
  (-1, 0, -6): the mesh has a face lit below sky 15 (the test fails if it has
  none), and the same Chunk with caves off has none.
- **The job.** `mesh_coarse` is deterministic for a Chunk and its
  neighbours, and the Server's existing landing and delivery tests pass
  unchanged.

## Performance

`lod/mesh_coarse` (an above-ground Chunk) is unchanged within noise; a new
`lod/mesh_coarse_covered` times a surface-band Chunk with a tunnel and
`lod/coarse_light` the rule alone, in absolute terms against the 0.3 to 0.4 ms a
coarse mesh costs. Load average is quoted before and after; a comparison within
5% is rerun interleaved against a build of `main` in its own worktree, never
through a fixed `/tmp` name. `meshing.rs` is not edited.

## Docs

ADR-0011's "Coarse light is full sky" consequence is amended in place: coarse
light is two-valued, sky above the heightfield and `COVERED_SKY` below it, and
the cost named there is now the pit and the lip. "Coarse levels store no light"
and "`light.rs` takes no new rule" stand. DESIGN.md's LOD section ("Light") and
`docs/architecture.md` say the same in a line. CONTEXT.md gets **Covered** only
if the code keeps the word.

## Done when

The tests above pass and were watched to fail; the Step 0 capture is repeated
with the rule in and described in the ticket's Comments (the mouth's interior
against the unchanged level-1 picture, and a plain-ground control); the
benchmarks are quoted; `./scripts/cargo-ci` passes in full; ADR-0011, DESIGN.md
and `docs/architecture.md` agree; and the ticket is `Status: done` in the
commit that finishes it.

## Tickets

| #   | Title                      | Blocked by |
| --- | -------------------------- | ---------- |
| 01  | Light the coarse LOD levels | -          |
