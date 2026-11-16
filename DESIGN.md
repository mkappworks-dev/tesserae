# Tesserae — design decisions

Status: pre-implementation. Settled 2026-10-11. Nothing here is code yet.

## What this is

A high-performance, moddable voxel **engine**. Games are content built on it.

The competitor is **Luanti** (formerly Minetest, 2010, C++/IrrlichtMt,
LGPL-2.1+), not Minecraft. Luanti already occupies "moddable voxel engine where
games are Lua content." Its weak points are its renderer and its Lua-only
performance ceiling. Those two are the differentiator.

### Target

- Modern lighting shipped **on by default**, not as a mod — the look Minecraft
  needs Iris plus a shaderpack to reach.
- 4km view distance by default, via LOD baked into the chunk format.
- Mods with no performance ceiling: WASM, not a scripting language.

### Non-goals

- Mobile. Web. Consoles. Not at 1.0, not later.
- Hardware ray tracing at 1.0. It **is** reachable on macOS (see "Verified
  constraints"), but `bevy_solari` cannot consume our meshes. Revisit post-1.0.
- Fine-grained destruction. 1m voxels, coarse destruction, accepted.

## Platforms

macOS first (dev and test machine), then Linux, then Windows. These three only.

## Settled decisions

| Decision             | Choice                                                                                                             | Why                                                                                                                                                                                                                                                                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Renderer             | CPU greedy meshing → rasterized triangles                                                                          | Large streamed worlds, 15 years of prior art. Not raymarched/SVO.                                                                                                                                                                                                                                                                     |
| Voxel size           | 1m default, per-world parameter                                                                                    | See "The voxel-size rule".                                                                                                                                                                                                                                                                                                            |
| Non-cube shapes      | **In scope** — stairs, slabs, fences, Luanti-style nodeboxes                                                       | Parity with Luanti's strongest feature. Expensive; see "Shaped voxels".                                                                                                                                                                                                                                                               |
| Engine               | Bevy 0.20                                                                                                          | Not 0.19 — superseded 2026-10-08.                                                                                                                                                                                                                                                                                                     |
| Bevy depth           | Full `bevy_pbr`, chunk materials via `MaterialExtension`                                                           | Shadows, SSAO, TAA, bloom, tonemapping, atmosphere, volumetrics come free and maintained. Hand-writing a renderer is a 12–24 month detour no user can see. Ethertum (543★, most active Bevy voxel game) does exactly this.                                                                                                            |
| Material model       | **PBR**                                                                                                            | Physical parameters + energy-conserving BRDF, so materials look right under any lighting. Minecraft vanilla is flat albedo × baked light — the reason it looks like 2011, and why LabPBR packs exist to retrofit PBR.                                                                                                                 |
| Forward vs deferred  | **Undecided — measure it.** Bevy's default is forward + depth prepass + clustered lighting; a deferred path exists | Clustered forward handles a few hundred lights. Deferred decouples light count from geometry, which suits voxels, but costs MSAA and still needs a forward transparency pass. Not a day-1 commitment.                                                                                                                                 |
| Graphics API         | wgpu → Metal / Vulkan / DX12                                                                                       | OpenGL is frozen at 4.1 on macOS (2010 spec, deprecated 2018, **no compute shaders**) and macOS is platform #1. Cost of wgpu: validation overhead and no backend-specific extras such as Metal RT.                                                                                                                                    |
| Anti-aliasing        | **SMAA** by default; a setting offers SMAA, MSAA 4x or Off                                                         | Bevy's 4x MSAA default was never chosen. At 1440p SMAA costs 0.5 ms less GPU than 4x, with edges close to it and crisp textures; MSAA 2x and FXAA cost the same as SMAA and look worse. SMAA leaves the held item unfiltered. Chosen on captures, 2026-11-02: `.scratch/2026-11-01-gpu-pixel-cost/issues/05-choose-anti-aliasing.md`. |
| View distance        | 4km                                                                                                                |                                                                                                                                                                                                                                                                                                                                       |
| LOD                  | Mip chain baked into the chunk format from day one                                                                 | Retrofitting is a storage-format migration plus a re-mesh of every saved world. Distant Horizons needs a separate datastore only because Minecraft's format has no LOD; we own the format.                                                                                                                                            |
| Multiplayer          | **Client-server architecture from day 1**, server runs in-process for singleplayer. Network transport deferred     | Minecraft Java's integrated-server model. "Add multiplayer later" is the most expensive retrofit in game dev because singleplayer code mutates world state directly. Build the split now, ship the transport later.                                                                                                                   |
| Modding              | WASM — `wasmtime`, component model, WIT interfaces                                                                 | No stable Rust ABI; dylib mods break on every rustc release, cannot be sandboxed, and force per-OS-per-arch builds onto modders. Lua would cap performance, which is the differentiator. Mods can still be written in Rust → `wasm32-wasip2`.                                                                                         |
| Code licence         | BSL 1.1, 4-year per-release Change Date, Change License Apache-2.0                                                 | **Verified valid.** BSL Covenant 1 requires compatibility with "GPL Version 2.0 **or a later version**"; Apache-2.0 is GPLv3-compatible, so it qualifies. CockroachDB 22.2 and Sentry 23.1 both shipped BSL 1.1 + Apache-2.0. The "Apache is GPLv2-incompatible" objection comes from mis-quoting the covenant.                       |
| Additional Use Grant | Unlimited use of the engine; restriction applies only to forks of the engine itself                                | Anything narrower bans the audience an engine exists for. Name the restriction explicitly in the grant text.                                                                                                                                                                                                                          |
| Media licence        | **CC0** for default and placeholder assets (CC BY 4.0 if attribution is wanted)                                    | **Not CC BY-SA.** Sharealike is viral on derived art, so a studio with proprietary art cannot use our textures as a base — friction exactly where an engine wants none. Brand protection is a **trademark on "Tesserae"**, not the asset licence. Fonts may be SIL OFL 1.1 (ADR-0005).                                                |

### Rejected, with reasons

- **Compute-shader DDA raymarching over a brickmap** (Teardown / dust / smoxel).
  Serves "raytraced, fine voxels, beautiful" better than anything else, and is
  what every project achieving that look actually did. Rejected in favour of
  staying mesh-based. Revisit only if the mesh path demonstrably cannot hit the
  visual target.
- **Bevy as ECS only + own wgpu renderer.** Correct for a raymarcher, wrong for
  triangles — `bevy_pbr` does most of that work already.
- **Custom Bevy render node.** The API this names was deleted in Bevy 0.19 (PR
  #22144, `RenderGraph` → systems) and the replacement churned again in 0.20.
  Across the whole Bevy ecosystem there are four custom voxel render pipelines;
  three are dead and the fourth runs a forked Bevy with no wgpu.
- **OpenGL.** No compute shaders on macOS, deprecated by Apple.
- **MIT / Apache-2.0 only** for code, and **CC BY-SA** for media. Considered.
- **Avian / Rapier for terrain collision.** General physics engines choke on
  millions of boxes. Sweep an AABB against the voxel grid directly. Keep a
  physics engine only for entities, if ever.
- **Hardware RT at 1.0.** Not because it is unavailable —
  `EXPERIMENTAL_RAY_QUERY` works on Metal — but because `bevy_solari` requires
  `StandardMaterial` and the exact vertex layout {POSITION, NORMAL, UV_0,
  TANGENT}, which a greedy mesher with a packed custom vertex format and a
  `MaterialExtension` cannot supply. Feeding it would mean giving up the custom
  material. A post-1.0 question.

## Crate layout

Three crates. With `bevy_pbr` doing rasterization there is no custom renderer
crate to own, so the middle crate is modding.

- **`tesserae-core`** — chunk storage, palette compression, LOD downsample,
  worldgen, greedy meshing, light propagation, fluid simulation, AABB-vs-voxel
  collision. No dependencies at all today, and no Bevy or wgpu ever: it uses
  `[i32; 3]` and a `LightVolume` trait rather than `glam`, and it has no
  scheduler, because streaming runs its functions as jobs on Bevy's task pools
  from `tesserae` (see
  [ADR-0007](./docs/adr/0007-streaming-jobs-run-on-bevy-task-pools.md)). `rayon`
  is still a candidate, for work that is data-parallel inside one job. Headless,
  unit-testable, benchmarkable under `criterion`. This is the hot path and the
  only part worth micro-optimising.
- **`tesserae-mods`** — Wasmtime/WASI host, versioned WIT contract, Manifest,
  Trust, Registry and pure Command validation. No Bevy or wgpu. The Server in
  `tesserae` owns scheduling and the pending Command queues.
- **`tesserae`** — the Bevy app. `MaterialExtension` materials for
  chunk/water/foliage, chunk streaming, camera, input, UI, saves, and the
  in-process server.

## The voxel-size rule

Voxel size is a per-world save parameter. It stays changeable only if all four
of these hold:

1. **Chunk dimensions are a compile-time constant in voxels** (32³). Storage
   never knows about metres.
2. **`voxel_size: f32` lives in world save data** and is the single conversion
   between voxel-index space and metres.
3. **Everything physical is computed in metres** — noise frequencies, gravity,
   step height, hitboxes, reach distance, fluid flow rate.
4. **The mod API exposes metres.** No mod may assume one voxel is one metre.

Tune one noise frequency in voxel units, or hardcode a one-voxel step height,
and voxel size is permanently fixed — every number in the game becomes secretly
calibrated to it.

Small voxels cost non-linearly; the arithmetic is in
[ADR-0003](docs/adr/0003-voxel-size-is-per-world.md).

## Shaped voxels

Greedy meshing merges coplanar faces of **identical full cubes**. A stair, slab
or fence cannot merge with a cube, so each becomes its own geometry. The mesher
is therefore two-pass: greedy-mesh the full cubes, then append per-voxel custom
geometry for shaped voxels, merging the quads of neighbouring shaped voxels that
lie in one plane, abut along a full edge and share Material and light (a floor
of slabs is a handful of quads).

Four consequences, all of which must be designed in from the start:

1. **A voxel is (material, shape, orientation)**, not a material id. Still
   palette-compressible, but the bit layout must budget for it now.
2. **Collision becomes per-shape.** The AABB sweep can no longer ask "solid?".
3. **Light propagation needs per-shape opacity**, not per-voxel.
4. **LOD must decide what a stair becomes at half resolution** — usually a full
   cube, or nothing.

Keep shaped voxels sparse in worldgen and cap them per chunk, or the mesher's
fast path stops mattering.

All four are resolved:

1. A Voxel is (Material, Shape, Orientation); a Shape is table data, so a Mod
   can declare one, and Orientation is `up * 4 + turn`, 0 to 23.
2. Collision is the Shape's own boxes, rotated, swept in metres.
3. Light crosses a face unless the Voxel entered seals its side or a shaped
   Voxel left seals its. There is no authored light-reduction value: that is a
   property of a Material, and belongs with them.
4. A stair at half resolution is a full cube or air by **volume**: a coarse
   Voxel is solid if its eight children's Shape volumes sum to at least half. So
   no level above 0 ever holds a shaped Voxel, pass 2 never runs there, and the
   mesher still does not need to know which level it is meshing. Step 6 writes
   the downsample; `ShapeTable::volume` is the datum.

A Chunk may hold at most `MAX_SHAPED_PER_CHUNK` (1024), the largest round number
that keeps a Chunk at the cap within +50% of a terrain mesh. The edit path
enforces it and the mesher does not police it.

Terrain places stairs and slabs where the ground steps one Voxel (slope
smoothing, see `.scratch/2026-10-12-worldgen-slope-smoothing/spec.md`): a stair
facing a single higher neighbour, a slab where two or more rise. It is gated by
a rise in **metres** (`SmoothingParams::min_rise_m`), so a World with Voxels
smaller than that is not smoothed. The generator keeps to half the cap,
`MAX_SMOOTHED_PER_CHUNK` (512), and leaves the other half to the player.

## LOD

Geometric rings, built in step 6
([ADR-0011](docs/adr/0011-coarse-levels-are-derived.md)). Level 0 covers
`view_distance_m` (192 m); level k >= 1 has Voxels and Cells `2^k` times larger
and an outer distance of `view_distance_m * 2^k`, clamped to `lod_distance_m`
(4096 m). The level count is `1 + ceil(log2(reach / view_distance_m))`, which is
6 at the defaults (levels 0 to 5), not the ~8 this section first assumed: that
was a 32 m full-detail radius, and the radius is the knob. Every level holds
about as many Chunks as level 0, so the total grows with the number of levels
and not with r². A Chunk is 32³ at every level and a Cell is its footprint in
metres, so everything is derived from metres and a World with smaller Voxels
gets the same rings.

- **The downsample.** A coarse Voxel is solid if its eight children's
  `ShapeTable::volume` sum to at least half, ties solid, and is always a full
  cube, so pass 2 never runs above level 0 and the mesher is not told the level.
  Its Material is the one with the most volume. A tie goes to the Material whose
  highest child is higher, then to the lower id: a grass cap over dirt is a
  four-to-four tie at every odd surface height, and by id alone every distant
  hill would be brown.
- **The hole.** Level k does not mesh where level k-1 does: it is a cylinder
  around the Eye, sized so that grown by one coarse Cell it still lies inside
  the finer level for any Eye position. Both levels floor the Eye to their own
  lattice, so measured from the anchor's centre the overlap is a Cell on one
  side and nothing on the other, which showed as a straight seam. At the
  defaults it is radius 1 and half-height 1 coarse Cell against a finer extent
  of 6 and 3. The overlap is the whole seam strategy: a finer face culled
  against a neighbour it did not draw has the coarse surface behind it, so there
  are no skirts and no stitching. The coarse level is still drawn under the
  finer one out to its edge, so the shader cuts a coarse fragment inside the
  finer level's guaranteed reach in metres, and inside its rows: without that a
  coarse surface up to a coarse Voxel too high stands over the finer horizon at
  ground level.
- **One camera per level.** Level k is on `RenderLayers` {k} and drawn by its
  own camera, the coarsest first and the only one to clear, each finer level
  over the last. A finer level's pixel wins whatever the coarse depth, which is
  what stops the coarse level poking through in the overlap. The near camera is
  level 0, the UI and everything that is not terrain. Two suns share a transform
  because a light's layers are also its shadow-caster mask: the one on layer 0
  casts shadows, the one on the coarse layers does not, and coarse terrain
  receives none.
- **Light.** A level above 0 stores none. The mesh job lights a face open sky
  unless it looks into air more than a Voxel under the terrain, which it lights
  dark (`coarse_light`, from the level's generator). See the ADR for what that
  gets wrong.
- **Fog.** Bevy's `DistanceFog` on all six cameras, built once from one
  `FogSettings` (a fog colour, a sky colour and `reach_opacity`, 0.88 by
  default), so a fragment is fogged the same whichever camera draws it. The
  density is derived from the reach in metres,
  `max(view_distance_m, lod_distance_m)`, so the opacity at the reach is
  `reach_opacity` at any Voxel size. The falloff is exponential squared because
  it starts slowly: under 2% to the join of levels 1 and 2 (384 m), where plain
  exponential already hazes the ground the Player stands on. The fog colour and
  the flat sky colour are a pair and not one value: the tonemapper runs in the
  shader after the fog and skips the clear colour, so the fog colour is the one
  that comes out as the sky. `apply_fog` is already the end of the chunk shader,
  so there is no shader change. `FogSettings` is live: changing the resource
  re-derives the clear colour and every camera's fog, so a Look pack sets it
  whenever it likes.

The per-level budget is measured, in Open items.

## What `bevy_pbr` does NOT give us

These are from-scratch subsystems in `tesserae-core`:

- **Torch and cave light.** Shadow maps handle sunlight. Thousands of point
  lights for torches is not viable on any GPU, so light is a per-voxel
  flood-fill (Minecraft's 0–15 model) baked into the mesh as a vertex attribute.
  Must be **incremental** — one torch re-floods a small region, not a chunk.
  Built in step 4. Two consequences worth carrying forward: light is part of the
  greedy mesher's merge key, because a merged quad has four vertices and cannot
  hold a level per Voxel it spans, which forecloses a light-only re-mesh path;
  and sky light drives `bevy_pbr`'s `diffuse_occlusion` and
  `directional_shadow_factor` rather than being multiplied into base colour, so
  albedo stays albedo and the BRDF keeps working.
- **Fluid simulation.** Water _shading_ is a `MaterialExtension`, roughly a
  week. Water _flow_ is a per-tick cellular automaton over millions of cells
  that must be incremental, chunk-boundary-correct and savable. One of the
  genuinely hard parts of a voxel engine.
- **Voxel collision.** AABB swept against the grid, per-shape. Not a physics
  engine.

## Modding boundary

The constraint that matters is **call frequency, not throughput**. Each WASM
instance has its own linear memory and crossing the boundary copies.

- A per-voxel `set_block()` call from WASM will be slow. Design the API as
  batched region operations and a command queue from the start.
- **Worldgen, meshing, lighting and fluids never run in WASM.** They stay
  native; mods parameterize them.
- wasmtime runs mod logic (block behaviour, entity AI, recipes) at roughly
  1.2–2× native, which does not matter for that work.
- Binary cost: wasmtime adds ~10–20MB; a mod is 50KB–2MB after `wasm-opt`.

## Look packs (our shaderpack equivalent)

In Minecraft a shaderpack is a zip of GLSL stages that Iris loads into fixed
pipeline slots. Here the boundary is **WASM declares, WGSL renders, the host
compiles** — WASM cannot put code on the GPU, so a pack ships WGSL as an asset
and the WASM module registers it by handle. A look pack provides:

- `MaterialExtension` WGSL for chunk / water / foliage
- post-process settings: bloom threshold, tonemapping curve, fog parameters
  (`FogSettings`), SSAO strength
- optionally, extra post-process passes

Bevy's asset system hot-reloads shaders, so pack authors edit WGSL and see it
live — something Iris does not offer.

Caveat to document: arbitrary WGSL cannot escape the GPU sandbox but **can**
hang the GPU into a device-lost reset. Look packs need a trust prompt like any
other mod.

## Licence consequences, accepted

Verified against the SPDX licence text, MariaDB's FAQ and announcement, and the
LICENSE files of eleven BSL adopters.

**Two naming traps.** The SPDX identifier is **`BUSL-1.1`**. `BSL-1.0` is the
_Boost_ Software License, which is OSI-approved and unrelated. The nixpkgs
attribute is `bsl11`, sitting next to an unrelated `boost`.

**Copy the SPDX text, not the web page.** `mariadb.com/bsl11` currently renders
Covenants 2–4 run together into one mangled sentence and prints **no Parameters
block** at all, despite the Terms referring to a grant "above". Take the body
from SPDX's `BUSL-1.1.txt` and the Parameters block from a real product LICENSE
file — HashiCorp's Terraform v1.6.0 is closest to our grant.

### What BSL actually restricts

- **Production use is the only prohibition.** Copying, modifying, creating
  derivative works and redistribution are all permitted from day one.
- **"Production use" is never defined** — the word appears twice in the licence
  with no definition clause, and it is absent from all three MariaDB FAQ pages.
  The entire boundary lives in our Additional Use Grant's prose. This is BSL's
  single biggest practical weakness.
- A derivative work may be redistributed but **cannot be relicensed** — not
  under MIT, not under anything.
- **BSL is not GPL-compatible before the Change Date**, so our code cannot be
  mixed into a GPL project until the date flips.
- A violation **auto-terminates rights for all versions**, not just the one
  breached. The License must be conspicuously displayed on every copy.
- The four-year rule is a per-version **backstop, not a cap on the written
  date**: it is "whichever comes first", so a later date is simply overridden.
- Covenant 4 is "Not to modify this License in any other way." (Redpanda's
  shipped text says "fifth anniversary" and omits the entire Covenants section;
  stated as a text comparison, not a legal conclusion.)

### Apache-2.0 as the Change License is the majority choice

Six of eleven surveyed adopters, including two Rust projects: **SurrealDB** and
**fuel-core**. Also Couchbase Server, Materialize, Akka, and historically
CockroachDB 22.2 and Sentry 23.1. MariaDB MaxScale is the only GPL-family choice
in the survey — from the licence's author.

MariaDB's own announcement settles intent: the Change License must be "GPL
compatible with **either GPLv2, GPLv3, or any other license that can be combined
properly with GPL software**." No authority has construed an Apache-2.0 Change
License as a covenant breach. The one adjacent public challenge (SpacetimeDB
#215) was about AGPL-3.0, and was closed without a licence change.

### Distribution, verified per policy

| Channel            | Verdict            | Evidence                                                                                                                                                                             |
| ------------------ | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **crates.io**      | **Yes**            | Only requires an SPDX identifier. `fuel-core` publishes with `license = "BUSL-1.1"`. SurrealDB uses `license-file` and so reports "non-standard" — prefer the identifier.            |
| **Arch `[extra]`** | **Yes**            | Five BUSL packages ship today (terraform, vault, consul, nomad, packer), tracking current releases.                                                                                  |
| **nixpkgs**        | **Yes, as unfree** | 27 BUSL packages, incl. current terraform. Needs `allowUnfree`; `redistributable = true` is what keeps it in the channel, but unfree is not built or cached by CI.                   |
| **Flathub**        | **Yes**            | Bar is redistributability. Spotify, Zoom and Obsidian ship as `LicenseRef-proprietary`.                                                                                              |
| **Homebrew cask**  | **Yes**            | Casks have no licence rule at all — `homebrew-cask` ships BUSL `consul` today.                                                                                                       |
| **Homebrew core**  | **No**             | Requires a DFSG-compatible licence. 0 of 8,648 core formulae are BUSL. Terraform was frozen at 1.5.7, then deprecated and removed.                                                   |
| **Debian**         | **No**             | DFSG §6. `packer` was **removed outright** rather than moved to non-free; zero BUSL packages exist anywhere in Debian. For `vagrant` Debian pinned the last pre-BUSL commit instead. |
| **Fedora**         | **No**             | `BUSL-1.1.toml` is `status = ["not-allowed"]` with no exceptions key. Extends to Copr, not just binary packages.                                                                     |

So Linux shipping is Arch + Flathub + nixpkgs + our own builds; macOS is a cask
or a direct download.

**GitHub will not display the licence.** `choosealicense`/licensee carry 47
licences and BUSL is not among them, so the API returns
`"license": {"key": "other", "spdx_id": "NOASSERTION"}` and the sidebar shows a
bare "View license" link.

**OSI names BUSL explicitly** under common reasons for rejection: "Conditional
licensing. Licenses with variable outcomes **like BUSL** that delay availability
of full software freedom won't be approved." MariaDB concedes the point in its
own FAQ.

### The alternative we are not taking

Sentry left BSL for the **FSL** and gave two reasons that apply directly here:
"the default non-compete time period is four years, which is a really long time
in the software world", and "the Additional Use Grant is the biggest problem" —
parameterising it makes the licence hard for compliance departments to approve.
FSL fixes the term at **two years** and permits only Apache-2.0 or MIT as the
change licence. Given our grant is maximally permissive anyway, FSL is the
closest competing choice if BSL's undefined "production use" becomes a problem.

### Other

- A CLA is required. (MariaDB's FAQ instead advises contributors license under
  new BSD or dedicate to the public domain; a CLA is the mainstream BSL practice
  and what we use.)
- Luanti is LGPL-2.1+: its ideas are usable, its code is not.
- `games/terra/` and `assets/` are deliberately **not** BSL — see ADR-0004.

## Road to version 1.0

Version 1.0 is a single-player Game, shipped as Mods on the Engine, that someone
can install, play for an hour, save, quit and resume, and then extend with a Mod
of their own. Steps 1 to 28 below are one sequence in suggested delivery order,
earliest first; a dependency is named where one exists. Network transport is not
in 1.0: ADR-0002 keeps single player on the same Server and Client split, so
multiplayer is a later transport swap, not a rewrite.

### Steps 1 to 9: core engine

1. `tesserae-core`: chunk storage + palette compression + greedy mesher, with a
   `criterion` bench on a worldgen-filled chunk. Headless, no window.
2. `tesserae`: spawn meshed chunks as `Mesh3d` with a `MaterialExtension`, fly
   camera. First time anything is on screen.
3. Worldgen + chunk streaming, behind the client/server split with the server
   in-process. Caves were added later, off the build order: tunnels in metres
   below a floor height, behind an exact range test that keeps most deep Chunks
   uniform. See `.scratch/2026-10-12-worldgen-caves/spec.md`.
4. Light propagation (incremental flood-fill). **Done.** Two channels packed
   into a byte per Voxel, in a parallel per-Chunk array rather than in the
   palette, baked into the mesh as a vertex attribute. Two solvers: one
   contiguous sweep for a Chunk arriving, and a Voxel-granular BFS for an edit,
   which is the half that makes "a torch re-floods a small region" true.
5. Shaped voxels — second mesher pass, per-shape collision and opacity.
   **Done.** A Shape is a row in a table of sixteenth-unit nodeboxes, and
   Orientation is one of 24 rotations
   ([ADR-0006](docs/adr/0006-shapes-are-table-data.md)). Everything else is
   derived from one rotated 16³ grid per Shape: which faces a Voxel seals, the
   geometry pass 2 emits, collision, and volume. Occlusion and light are per
   face. The Server's edit path is live, with a ray as the Client's only intent,
   and `collide.rs` has a metre-valued sweep and ray. Glass, leaves and water
   were added later, off the build order, as a `MaterialTable` that carries
   emission, light dimming, occlusion and render class
   ([ADR-0009](docs/adr/0009-materials-are-table-data.md)). See
   `.scratch/2026-10-12-material-table-transparency/spec.md`. Connected Shapes
   (a fence growing arms) and collision up to a Voxel taller than the cell
   follow in the same step, with connections derived from the neighbours and
   never stored
   ([ADR-0010](docs/adr/0010-connections-are-derived-not-stored.md)).
6. LOD mip chain out to 4km. **Done.** Six levels (0 to 5) around the Eye, each
   a ring of Chunks of its own Cells with a hole where the next finer level
   draws, generated for the purpose and never stored or edited
   ([ADR-0011](docs/adr/0011-coarse-levels-are-derived.md)), and one camera per
   level. At the defaults levels 0 to 4 each hold 2167 Chunks and level 5 holds
   1243, with 127.9 MB of vertex and index memory (41.5 MB of it level 0's), the
   median flight frame is 5.6 ms on an M5 at 1080p offscreen, and the furthest
   drawn Chunk ends 5174 m out. Level 0 is drawn 0.3 s later while the distance
   fills, which takes 11 s. Distance fog on all six cameras is not measurable
   against that frame: interleaved runs on a quiet machine put the paired
   difference with and without it at 0.0 ms, which bounds the cost under 0.2 ms.
   See `.scratch/2026-10-13-phase-6-lod/spec.md` and
   `.scratch/2026-10-14-distance-fog/spec.md`.
7. `tesserae-mods`: WASM host, WIT interfaces, batched command queue. **Done.**
   Declarations and exact-build Trust precede App creation; asynchronous Mod
   Jobs return validated batches to the Server's fair, bounded queues
   ([ADR-0012](docs/adr/0012-mods-declare-once-and-command-in-batches.md)). On
   the 10-core M5, calibration measured 41.5 million fuel/ms for the heavy mix
   and 63–73 million for the light loop (load 3.3–4.3). `CALL_FUEL` is 200
   million, `YIELD_FUEL` 10 million, `MAX_CALL_WALL` 250 ms. With patched
   Wasmtime/WASI 49.0.4 and Cranelift 0.136.4 (RUSTSEC-2026-0378/0379),
   interleaved release process-CPU measurements gave empty ticks of
   34.48/34.82/35.10 µs and batched ticks of 34.24/35.88/85.16 µs for
   16/256/4096 placements (load 9.01→8.55). Native validation cost
   0.06/0.48/7.27 µs at those sizes (load 9.01→8.55). The naive benchmark cost
   0.93/11.57/178.99 µs (load 9.01→8.55), but omits the production tick's
   deadline thread: no flat per-command cost or small-batch speedup is
   established. At 4096 placements the naive path is 1.94 times tick plus
   validation (load 9.01→8.55). `MOD_VOXELS_PER_TICK` was **2** while each write
   re-flooded light on its own, replacing the initial 256: the median worst
   drain was 0.296 ms (load 10.43) versus 10.585 ms at 256 (load 9.52–9.92), and
   a full 32³ fill took 16384 uncontended, lit ticks, about 256 s at 64 Hz.
   Ticket 04 of `.scratch/2026-11-03-bulk-relight` batched the drain: 96 writes
   a tick, an emitter counting as 6, 4096 looks; lumen's 19,712 Voxels at 0.5 m
   drain in 3.1 s with a worst drain tick under 2 ms, against 8.4 ms for the old
   path at the same 96 a tick. Final live filling/control worst ticks were
   5.3/3.7 ms (respective loads 5.04→6.07/5.12→6.12) and 3.6/6.1 ms
   (4.79→5.52/5.52→6.74). These observations support the conservative cap, not a
   universal latency bound; presentation work differs, so live CPU totals are
   not equal-work throughput. The full fixed-tick metric also replaces the old
   streaming-only frame sum. Evidence and qualifications are in tickets 04, 05
   and 07 of `.scratch/2026-10-14-phase-7-mod-host/`. The MIT terra exercises
   the contract at 1 m and 0.5 m without a direct engine dependency.
8. Fluid simulation. **Done.** Sources feed flowing fluid that falls first and
   spreads a declared reach in metres, renewing it after each declared drop and
   receding without supply; water waterlogs Shapes through Voids derived from
   their geometry; Mods declare fluids, contact reactions and fuel in a 0.2
   contract line that 0.1 components reach through an adapter; local heat
   exposure through open space ignites fuel, which burns for a declared time to
   a residue and is put out by an extinguishing fluid touching it. Fluid and
   fire live in an Overlay beside the packed Voxel with one reading of both
   ([ADR-0013](docs/adr/0013-fluid-and-fire-live-beside-the-voxel.md)), and the
   simulation advances in logical steps under count bounds
   ([ADR-0014](docs/adr/0014-simulation-advances-in-bounded-logical-steps.md)).
   Only held level-0 Chunks simulate; level 1 shows their fluid and fire, and
   levels 2 and up stay generator-only
   ([ADR-0011](docs/adr/0011-coarse-levels-are-derived.md)). Defaults, measured
   at equal physical fixtures at 1.0, 0.5 and 0.25 m: `LANDED_PER_STEP` 256,
   `EMITTERS_PER_STEP` 4, an Agenda of 16384 entries plus a reserve of 4096,
   4096 Voxels rescanned a step, 1024 evaluations a Job and 4 Jobs in flight,
   and a light solve made on the tick after `SPOILT_BEFORE_ON_TICK` (4) spoilt
   solves. Terra's water reaches 7 m at 4 m/s sideways and 8 m/s down, its lava
   3 m at 0.75 and 1.5 m/s. Water was built in at the time and is the terra's
   own since step 8.1 ([ADR-0015](docs/adr/0015-water-is-game-content.md)).
   **These Server figures are at the `dev` profile** (opt-level 1, what
   `cargo dev` runs), which overstates a release build, and were taken on a
   shared host at one-minute loads of 1.5 to 7.6: the worst main-thread tick was
   11.2 ms of thread CPU (lava draining at 0.25 m) against the 15.625 ms of a 64
   Hz tick, with no tick over it in any scenario and every player Edit landing
   on its tick. At 8 emitters a lava step reached 13.75 ms, which is why the
   bound is 4. In release a flow evaluation costs 0.3 to 0.5 us, a fuel
   evaluation up to 8.9 us at 0.25 m, and a Chunk-sized rescan about 9 ms of
   worker CPU. A dry, unburning Chunk's meshing is 15 to 45% faster than before
   the phase and its bulk light within noise of it, in release thread CPU;
   registering the built-in Shapes costs 8 to 15% more (4.8 ms), which is the
   Void derivation itself. A quota bounds work, not time. Evidence: tickets 09
   and 11 of `.scratch/2026-10-22-phase-8-fluid-simulation/`, which the MIT
   terra exercises at all three sizes.

   **Step 8.1: water is Game content, and the player places anything declared.
   Done.** At this step the engine built in no fluid and retired id 7; step 8.2
   below retires the remaining solids. Mod Materials still start at 8. A World
   with no Game became dry ([ADR-0015](docs/adr/0015-water-is-game-content.md)).
   A Game's lakes name their fluid in contract line 0.3, which 0.2 and 0.1
   components reach through adapters; a 0.2 Game that asks for standing water
   opens dry with a warning. Terra declares `terra:water` and lakes at 25.5 m,
   clear of its stage. A player ignites fuel and puts fire out through the Edit
   path, under the same rules and messages as a Mod's Commands, and a 0.3 Mod
   sees both as Edits. The hotbar is built from a content catalogue the Server
   publishes, and a Tab menu puts any declared solid in any of its Shapes, any
   fluid or fire into a slot. `cargo dev` loads the in-repo Terra once it is
   built, trusted for that run only. Against main, interleaved thread-CPU-time
   runs put the meshing, collide, LOD and fluids benches within noise; the one
   steady outlier, `greedy_mesh/terrain_water` at +7%, is code layout, since its
   mesher is unchanged and the same water at id 7 and id 8 costs the same in one
   binary. Evidence: `.scratch/2026-10-25-phase-8.1-game-content/`.

   **Step 8.2: no built-in content but air. Done.** Air is the only engine
   Material; ids 1 to 7 are retired and Mod ids still start at 8. The Game names
   optional ground on the amended unreleased 0.3 contract; older lines warn and
   have no ground. Terra declares its solids. Hotbar defaults and the gallery
   read declared content. A single remove action, an item held outside the
   slots, square menu and hotbar model tiles, and Server-answered aim previews
   complete the player's content controls. Tiles share a content-only model
   atlas, with one Game-supplied texture shared by World surfaces, previews and
   translucent placement ghosts, plus a model that follows slot drags. The final
   texture captures were reviewed and deleted; the controls follow-up was also
   reviewed and its capture folder deleted. Help and diagnostics toggle
   independently, targeting stays stable during movement, and X selects empty
   hand with familiar mouse bindings. Deliberate removal has a progress bar and
   a Mod-defined Material hold time; guarded Server undo/redo restores direct
   player edits. Removal progress, placement cues and help were also reviewed
   and their capture folder deleted. All 902 tests pass, and the interleaved
   thread-CPU comparison against phase 8.1 main covers 115 rows. Same-binary
   probes isolate id moves from layout shifts; unchanged meshing and coarse
   derivation remain comparable. Results, outliers and exceptions are recorded
   in ticket 13 of `.scratch/2026-10-26-phase-8.2-no-built-in-content/`. See
   [ADR-0016](docs/adr/0016-only-air-is-built-in.md).

9. Look-pack API, water shading, visual polish. Polish includes the sky and its
   sun, a day/night cycle, and cave lighting that reads as dark and lit by
   torches instead of flat ambient. Sky and time of day are World or Game
   settings, not a function of the seed. Biomes are generator behaviour and
   belong to step 13; they are listed there because they change what the sky and
   fog should be.

### Steps 10 to 29: engine systems and game content

Each step has an Engine part, the mechanism, and a Game or Mod part, the data
that uses it. The Engine part always comes first, crosses the WIT contract, and
ships no content: ADR-0016 keeps air as the only built-in Material. The
controls, movement, save, camera and input work already in flight is tracked in
`.scratch/2026-10-27-gameplay-performance-and-controls` and is not repeated.

| #   | Step                    | Engine adds                                                                                                                  | Game or Mod supplies               | Depends on | Status          |
| --- | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- | ---------- | --------------- |
| 10  | Modding SDK             | Data-only declarations, folder loading, reload loop, SDK docs and WIT compatibility policy                                   | Example Game                       | -          | Done            |
| 11  | Use action, first tool  | Server "use" verb, target validation, success and refusal feedback (tickets 20-21)                                           | Which tools exist and what they do | -          | First tool done |
| 12  | Dropped items, pickup   | Entity system: Server simulation, pickup, save                                                                               | Item definitions                   | -          | Done            |
| 13  | Biomes, richer worldgen | Biome and climate seam, generator version pinned in the save                                                                 | Biomes, ores, surface features     | -          | Done            |
| 14  | Audio                   | Server-driven events, metre attenuation, mixing, Soundscapes and Audio settings                                              | CC0 Terra sounds and music         | -          | Done            |
| 15  | Inventory               | Server-owned transactions that compose with the undo journal, save                                                           | Stack sizes, item rules            | 11, 12     | Done            |
| 16  | Crafting                | Recipe schema, station interaction, Use on Voxels (doors, containers), UI host                                               | Recipes, stations                  | 15         | Done            |
| 17  | Terrain and water       | Generator version 3: landforms and oceans, rivers and lakes, caves and the underground, Biomes by altitude and coast (17a-d) | Terrain and water per Biome        | 13         | Not started     |
| 18  | Plants                  | Growth on the Server tick, placement rules, harvesting                                                                       | Plant data                         | 13         | Done            |
| 19  | Map, compass            | Server-provided summaries (the Client never reads World storage), map view                                                   | Map style, markers                 | 13         | Done            |
| 20  | Structures              | Template format, deterministic placement by worldgen and tools                                                               | Templates                          | 13         | Done            |
| 21  | Survival loop           | Health, damage, hunger, death, respawn, game modes                                                                           | Numbers and rules                  | 15         | Done            |
| 22  | LOD features            | Trees and Structures drawn at coarse levels as sparse impostors from the same pure draws, and a ring quality curve           | Larger default view distance       | 6, 18, 20  | Not started     |
| 23  | Blueprints and Sites    | Blueprint Items that lay a Site, which builds itself over time from Items brought to it                                      | Blueprints                         | 15, 20     | Not started     |
| 24  | Weather                 | Weather state and tick, particles, sky hooks                                                                                 | Weather types, climate rules       | 13, 17     | Not started     |
| 25  | Mobs, animals           | Entity AI hooks, spawning, replication shape designed against transport                                                      | Behaviours, spawn tables           | 12, 13, 21 | Not started     |
| 26  | NPCs                    | Persistent identity, dialogue and trade plumbing                                                                             | NPC definitions, dialogue          | 15, 25     | Not started     |
| 27  | Logic and machines      | Wiring and automation, then moving and flying Voxel bodies (see below)                                                       | Machine parts, logic components    | 12, 20     | Not started     |
| 28  | Creation tools          | Structure, material and texture editors, screenshot and replay mode                                                          | Content made with them             | 10, 20     | Not started     |
| 29  | Chat and commands       | Command registry, parser, permissions                                                                                        | The commands                       | -          | Not started     |
| 30  | Release hardening       | Final 1.0 contract freeze, macOS bundle, first-run, save migration tests, diagnostics, CI budgets                            | -                                  | all        | Not started     |

Notes:

- **Step 11** is done for the first tool. Use is a Server intent of its own,
  answered and never applied, that enters no history; a tool is a hand state, so
  holding one makes Place and Remove its verbs and leaves undo and redo live. A
  Game declares tools once, as a name and a closed kind in the unreleased 0.3
  Declaration, and the Server runs the kind in native code, so no Mod is called
  when a tool is used. Terra declares one, Measure: it reads two corners of
  drawn Shapes in metres and Voxels and edits nothing. Taking a Voxel into an
  inventory is step 15, and using a Voxel (a door, a container) is part of
  step 16.
- **Step 13** is done. Climate (temperature and humidity, in metres) chooses a
  Biome, and a Biome owns ground Materials, ores, a cave scale and a sky and fog
  pair; it never changes surface height. A World pins its Generator version and
  a snapshot of its Biome table, so a new biome never changes an existing
  World's terrain for the same seed. The Server sends the Client the sky and fog
  pair for where the Eye is. All five demo Games declare three Biomes and one
  ore. A Biome may also declare surface features, Materials stood one Voxel
  above its surface at a mean spacing in metres; they are pinned in the
  snapshot, so a World saved before them keeps its bare ground, and coarse
  levels leave them out. See
  [ADR-0022](docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md).
- **Step 15** is done. Each Player has a Server-owned Inventory, a 27-slot
  backpack and a 9-slot Hotbar of Stacks, whose rules are a pure module in
  `tesserae-core`; the Client sends intents and draws what the Server sends.
  Pickup fills it and leaves on the ground what does not fit. In every mode,
  Place spends one Item from the selected slot, a Layout is paid up front or
  refused, and removing a Voxel drops its Item. Undo pays and refunds Items in
  the same Server transaction, by the mode each entry was made in, and anything
  that does not fit drops at the Player. A Material with no declared Item has an
  implicit one, so every placeable Material is an Item and drops as one: a fluid
  source drops its Item when removed, flowing fluid never does. Carried is
  retired: a save's totals pack into slots once. In Creative the Catalogue sits
  beside the Inventory and gives Items. Games declare stack limits (`max-stack`,
  64 when absent). See
  [ADR-0023](docs/adr/0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md).
- **Step 16** is done. A Game or Mod declares Recipes as data (1 to 9 inputs,
  one output, and a required `station` that is `"hand"` or a Station's tag) and
  Usable Voxels as `voxel-uses` rows of a closed kind: a Door (a closed and an
  open Material), a Container (1 to 54 slots) or a Station (a tag). The Server
  runs each kind natively. Use runs a held Tool, else the Usable Voxel aimed at
  in reach, else eats. A Door is a size in metres (Terra's 1 m by 2 m, its Gate
  1 m by 1 m), placed whole as thin panels that turn on their hinge, and toggles
  outside the undo journal. Any `voxel-uses` row may fix the Shape its Material
  places, and that Shape is left out of the Player's Shape choices; Terra's
  Workbench is a table. A Station session allows its tag's Recipes; a
  Container's Stacks live outside the Chunk, are saved in the World commit, are
  refused to the Player's Remove and history while full, and spill when anything
  else removes them. Crafting is instant and never journalled. The Inventory
  window gains a Crafting tab and a compact item details panel; Terra ships a
  Workbench, Chest, Door and Gate with CC0 sounds, and Wood on temperate grass
  so Survival can craft them. The mockup's `/blocks` gallery previews every
  Terra Material from its own textures. See
  [ADR-0026](docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md)
  and the 2026-11-13 amendment to
  [ADR-0023](docs/adr/0023-the-inventory-is-server-owned-and-pays-inside-the-undo-journal.md).
  Spec and tickets: `.scratch/2026-11-13-phase-16-crafting/`.
- **Step 18** is Done. A Game or Mod declares Plants as data, of five kinds the
  Server runs natively: `crop`, `tree`, `column`, `spreader` and `climber`. A
  Plant's state is its Material, so nothing is stored per plant and the save,
  the Overlay and the Agenda are unchanged. Growth is a Chunk-sampled tick on
  the Logical step: each held Chunk that holds a Plant Material draws
  `PLANT_SAMPLES` positions, a Sampled Voxel rolls against its Stage's
  `mean-seconds` (no faster than the 16 s Sample interval), and a passing roll
  grows it if light, Climate, water and soil hold, otherwise it waits and never
  dies. Trees are parametric, grow whole from a sapling and fell top-down when a
  trunk is cut, where only grown Materials take part, so a cabin of placed logs
  stays. Plants carry flat `cross` and `plane` Shapes, fruit is a small box
  Shape that is broken to harvest (the tree grows new fruit elsewhere on its
  canopy while it stands), soil and water are Material tags, and a Biome may
  list 16 surface features, Plants included, with vines hung from trees by
  worldgen only. The shared Mods `flora`, `crops` and `aquatic` declare the
  vegetation every Game places. Not shipped: falling trees as bodies (step 27),
  trees as Templates (step 20 kept them parametric), leaf decay and weather
  inputs (step 24). Measured by the benchmark pass (Apple M5, thread CPU time,
  interleaved rounds, load average 8 to 9,
  `cargo bench -p tesserae-core --bench plants` and the `--ignored` rig in
  `server/fluids/plants/tests/bench.rs`): `PLANT_SAMPLES` stays 32, so a Sample
  tick costs 0.01 us a Chunk with no Plant Material, 0.56 us a crop-meadow Chunk
  and 0.34 us a forest Chunk, and `PLANTS_LANDED_PER_STEP` stays 64 (288 us a
  step when saturated, 4.5 us a landing). `TREES_PER_STEP` is 1: an oak of 4 to
  6 m lands whole in 0.16, 0.86 and 6.1 ms at 1.0, 0.5 and 0.25 m, so two a step
  would be 80% of a 15.6 ms step at 0.25 m. `FELLED_PER_STEP` stays 32 (0.11,
  0.11 and 0.38 ms a step). Meshing 1024 `cross` Voxels costs 350 us against 209
  us for the bare terrain, below the 478 us of 1024 stairs, so flat Shapes stay
  uncapped; 4096 cost 541 us. A surface Chunk of a 16-feature Biome generates in
  0.42, 0.24 and 0.20 ms against 0.12, 0.08 and 0.08 bare. Branching trees
  (ticket 18; limbs, forks, lean and lobes in metres, the shared species of
  `flora` all branch) cost more: the shared oak is 6590 Voxels at 0.25 m against
  3341 and lands in 0.29, 1.58 and 10.2 ms at 1.0, 0.5 and 0.25 m against 0.17,
  0.86 and 6.1, 65% of a 15.6 ms step, so `TREES_PER_STEP` stays 1 (it cannot go
  lower); a felled step of 32 Voxels costs 0.12, 0.14 and 0.74 ms; a surface
  Chunk of the 16-feature Biome with three branching trees generates in 0.73,
  0.41 and 0.54 ms against 0.43, 0.25 and 0.20 with plain trees. Density and
  groves (ticket 19): a feature may declare a `clump`, a scale in metres and a
  strength whose low-frequency field multiplies its chance at a mean of 1, and
  the shipped Games tune every Biome to the approved classes (dense trees 3.6 to
  4.7 m apart, moderate 5.7 to 8 m, sparse 15 to 67 m, measured over a sample of
  Chunks). Clumping every feature of the worst-case 16-feature Biome costs a
  surface Chunk 0.61, 0.31 and 0.22 ms at 1.0, 0.5 and 0.25 m against 0.43, 0.25
  and 0.20 without (interleaved thread CPU time, load average 7 to 8): one noise
  evaluation per clumped feature per column that can hold a feature. Harvest is
  breaking (Use harvests nothing) and a standing fruit tree re-bears: each of
  its canopy Voxels Samples at a mean of `regrow-seconds` times the tree's
  expected canopy cells, so the tree grows one unripe fruit about every
  `regrow-seconds`, up to `density` of its canopy. Measured (load average 8 to
  12, the `--ignored` rig, 64 held Chunks of 16 trees): a bare fruit forest
  costs 32.8 us a step against 23.6 for a forest of fruitless trees, so the
  re-bearing site is about 0.14 us a Chunk, and at generation's density it is
  87.9 us with 4.7 ripenings landing a step; the expected canopy count costs 8,
  26 and 176 us once per Plant and Voxel size for the branching oak at 1.0, 0.5
  and 0.25 m (1.7, 7.9 and 55 us plain), then is cached. See
  [ADR-0027](docs/adr/0027-plants-are-declared-kinds-the-server-grows.md) and
  the 2026-11-14 amendments to
  [ADR-0006](docs/adr/0006-shapes-are-table-data.md),
  [ADR-0009](docs/adr/0009-materials-are-table-data.md) and
  [ADR-0022](docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md).
  Spec and tickets: `.scratch/2026-11-14-phase-18-plants/`.
- **Step 19** is done. Each Player has a Server-held Map of the surface they
  have explored: per 2 m column sample, the height, top Material and Biome as
  they were when seen, captured from loaded, lit ground near the Eye and never
  regenerated from the seed, so builds and lakes show as of the last visit. The
  Server sends 128 m tiles under a per-step budget and the Client colours them
  from the Material palette; the Client never reads World storage. The Compass
  reads the heading the Server publishes in the pose, North is -Z in every
  World, and a minimap and a full-screen Map (keys M, N and B) draw from the
  Client's cache. Waypoints (at most 64 a Player) are placed and removed by
  intent on Explored ground, and a Game may declare Marker kinds from a closed
  icon set and the kind a death leaves. A Game may forbid the Map and the
  Compass with `[world.map]`. The Map saves as its own compact section, about 6
  percent of its raw size, and a save without it opens empty. See
  [ADR-0028](docs/adr/0028-the-map-is-a-server-held-per-player-capture.md). Spec
  and tickets: `.scratch/2026-11-15-phase-19-map-compass/`.
- **Step 20** is done. A Game or Mod declares Templates as data in the
  unreleased 0.3 Declaration: a grid of Template cells in metres, each a
  Material, a Shape and an Orientation by full name, or Keep, Air or `ground`
  (the local Biome's surface Material, its soil below the Ground layer), with a
  Ground layer, an optional foundation and a level tolerance. A World samples a
  Template at its own Voxel size, so a Structure has the same footprint in
  metres at 1.0, 0.5 and 0.25 m
  ([ADR-0003](docs/adr/0003-voxel-size-is-per-world.md)'s amendment). A Biome
  lists Structures and Settlements; worldgen stands them as pure functions of
  the seed and metres, whole across Chunk borders, on level, dry, un-cut ground,
  and trees, scatter and crop fields keep off them. A Settlement lays roads from
  a centre and Plots beside them, each building's door facing its road, cuts and
  fills each Plot level by at most 2 m and builds steep roads as stairs and
  slabs. Every Material a Template names must be Obtainable in the World, or the
  World is refused. A Creative Player's `place` Tool opens the Template list,
  turns a Structure with R and places it whole as one undo group, from a ghost
  the Server tints Template cell by Template cell. Every Game declares fifteen
  Templates, the `place` Tool and a Settlement (Terra also a town), generated
  from the approved mockup by `mockup/export-templates.mjs`, and the Rust
  starter a hut. Terra's, Meadow's and Tidewater's bridges declare `spans` and
  stand only across a gap, a valley or water, between two dry landings. A World
  freezes the Templates and Settlements it uses in its generation snapshot, so
  an old World generates what it did, with no new Generator version. How
  Structures and Settlements are spread and their roads run is described in one
  place, the authoring guide's
  [Where they stand](docs/authoring-guide.md#where-they-stand). See
  [ADR-0029](docs/adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md).
  Spec and tickets: `.scratch/2026-11-15-phase-20-structures/`.

  Not shipped, or shipped short:
  - Blueprints and Sites are step 23. Container loot, entities, fluids,
    weathering variants, mirroring and Structures at coarse LOD levels are out
    of scope.
  - A placement that would change more than the undo journal's 131,072 Voxels
    (`RECORDED_LIMIT`) is refused with `TooBigToUndo` rather than landed without
    an undo group; at 0.25 m a 32 m building can. An undo or redo still lands
    whole in one tick, and Mods hear every Voxel of it as an Edit event: past
    1,024 (`MAX_EDIT_EVENTS`) in one tick, the rest wait in order and are
    released 1,024 a tick, so a Mod called every tick misses none.
  - R is both Respawn and Turn Structure (`TurnStructure`); while the `place`
    Tool is held it turns the Structure and cancels Respawn, and Settings lets
    the two share that one key.
  - Against the approved mockup, the Client's Template list is flat (no Homes,
    Trades groups or per-Template notes, which nothing declares), the HUD card
    has no isometric thumbnail (the ghost is in the World) and no buttons (the
    cursor is grabbed in play; G and right mouse do the same), left mouse does
    not cancel (Remove is taken while the Tool is held, and there is nothing to
    cancel), and refusals read the Server's `RejectReason` messages rather than
    the mockup's wording.
  - The Tool lays no foundation and plays no Sound for a placement.

  Measured (ticket 18; Apple M5, thread CPU time, interleaved rounds, load
  average 1.6 to 1.7, `cargo bench -p tesserae-core --bench structures`,
  Templates and Settlements built from core types on 12 m hills with caves and
  smoothing). The agreed margin: Structures or a Settlement add at most 0.25 ms
  to a surface Chunk at any Voxel size, whether the Chunk is the first to lay
  out its lattice cells (cold) or a later one (warm), which is below the 0.30 ms
  the 16-feature Biome's Plants add at 1 m. Every Chunk a Structure or
  Settlement reaches used to accept or lay it out again, which put a Template at
  the cap (64 by 16 by 64 m, every Template cell written) at 0.63, 0.59 and 0.57
  ms a Chunk at 1.0, 0.5 and 0.25 m and a Settlement at the cap (64 m radius, 24
  Plots) at 0.50, 0.37 and 0.36 ms; so each generator, and every clone streaming
  makes of it, now shares a bounded cache of what each lattice cell stands
  (16,384 Structures, 4,096 Settlement centres and 32 laid-out Settlements,
  forgotten whole when full), which keeps generation a pure function of the seed
  and metres in any Chunk order and on any thread. Per Chunk, warm then cold
  against bare: a 7 m house 64 m apart costs 0.21 and 0.24 ms against 0.21 and
  0.20 at 1 m, 0.13 and 0.15 against 0.11 at 0.5 m, and 0.10 and 0.12 against
  0.06 at 0.25 m; 14 m apart (the densest it may be) 0.30 and 0.37 against 0.23,
  0.15 and 0.19 against 0.08, and 0.14 and 0.16 against 0.07; the Template at
  the cap 0.26 and 0.36 against 0.25, 0.21 and 0.28 against 0.13, and 0.20 and
  0.26 against 0.08 (the dearest, 0.18 ms over bare); a 32 m village of 12 Plots
  0.23 and 0.28 against 0.21, 0.17 and 0.22 against 0.11, and 0.15 and 0.20
  against 0.08; and the Settlement at the cap 0.24 and 0.31 against 0.23, 0.11
  and 0.18 against 0.09, and 0.10 and 0.17 against 0.07. Ratios to bare taken
  round by round reproduce within a few percent at load average 9 to 12. Sky and
  deep-rock Chunks beyond the band keep their early-outs (0.01 us with or
  without Structures). A World without Structures is not free: an interleaved
  A/B of separate builds puts bare generation at 2.0, 5.1 and 8.5% above main at
  1.0, 0.5 and 0.25 m (about 5 us a Chunk), the same before and after this
  ticket; it bisects to the Settlement levelling commit, which changes nothing
  on the bare path, and the machine code of `Worldgen::chunk` shows its column
  loop re-addressing the Chunk builder from the stack rather than a register, so
  it is code generation and stays open.

  Settlements from hamlets to towns (ticket 24) raised the caps to a 160 m
  radius and 96 Plots, and the margin is revisited for them: warm, a town at the
  new cap stays within 0.25 ms (0.21, 0.13 and 0.12 ms a Chunk against 0.19,
  0.08 and 0.06 at 1.0, 0.5 and 0.25 m), but cold it costs 0.45, 0.37 and 0.36
  ms, 0.26 to 0.30 over bare. Cold charges its one layout, about 2.5 ms, to the
  9 Chunks timed; it is cached per lattice cell and such a town reaches over a
  hundred surface Chunks, so the margin for a Settlement wider than the old 64 m
  cap is 0.35 ms cold, and 0.25 ms warm as before. Settlements of 64 m or less
  lay out exactly as before.

- **Step 21** is done. A World has a default game mode and each Player their
  own, Creative or Survival; Edit World's changes wait in the sidecar until the
  Server applies them at the next open
  ([ADR-0024](docs/adr/0024-game-mode-is-per-player-with-a-world-default.md)).
  In Survival the Server keeps health, hunger and air on its 64 Hz tick, in
  metres and seconds: fall damage above a safe height, drowning, contact damage
  from a Material that declares it, hunger that drains faster while sprinting,
  regeneration while hunger is high and starvation at zero. Food is an Item with
  a food value, eaten with Use. At zero health the Player drops their Inventory
  where they fell unless the Game keeps it, loses their undo history, and sees a
  death screen that counts down to the drops' despawn; Respawn is at the World
  spawn with full Vitals. The Game declares its Survival rules (maximum health,
  hunger and air, safe fall height and damage rates, hunger drain, the
  regeneration threshold and `regeneration-per-s`, `keep-inventory`); a Mod may
  not, and a Game that declares none plays on engine defaults. Every demo Game
  has its own food as a surface feature of its Biomes: the terra melon, meadow
  pumpkin, ember pepper, tidewater coconut and lumen glowberry. Lava hurts on
  contact in terra and in `hearth`. Spec and tickets:
  `.scratch/2026-11-05-phase-15-menus-and-inventory-ui/`.
- **Step 10** is the gate for "ready for Mods" and has no dependency on the
  later gameplay systems. Folder loading and versioned WIT contracts already
  exist; this step completes the authoring workflow, starting with data-only
  Mods. Its tickets are built and checked by tests, including both starters
  copied outside the checkout (`docs/authoring-guide.md`), and the
  edit-and-restart loop was run by hand in the real window for a data change, a
  Rust change, a failure and its correction (ticket 08). It establishes how
  contracts evolve compatibly while later systems add APIs. The final 1.0
  contract freeze belongs to step 30, after those systems are implemented.
  `mods/` holds fifteen shared data Mods and `games/` four small Rust Games
  built on them (`docs/demo-games.md`); `terra` remains the one self-contained
  Game.
- **Step 17** is not started. It reworks the whole of worldgen, so it comes as
  Generator version 3; a World made under version 2 keeps its terrain and
  recorded digests. Today the surface is one height noise, caves are one tunnel
  field, and a Biome, picked by temperature and humidity, never changes height.
  Step 17 replaces that with a layered generator, delivered in four parts:
  - **17a Landforms and oceans.** Continental-scale fields (how far inland, how
    eroded, peaks against valleys), domain-warped so nothing reads as noise,
    shape oceans (shelf, slope, deep floor, trenches), coasts with beaches,
    cliffs and stony shores, archipelagos, plains, hills, plateaus and mesas,
    canyons, badlands, volcanic cones and mountain ranges with ridges and
    snow-capped peaks. A 3D density near the surface allows overhangs, arches
    and sheer cliffs.
  - **17b Water.** A sea level, lakes in basins, and rivers and streams found by
    a drainage pass over a coarse lattice, from the mountains down to a lake or
    the sea, carving valleys, deltas and waterfalls where they drop; springs and
    a water table. Each region's drainage is computed once from the seed and
    cached, so generation stays a pure function in any Chunk order.
  - **17c Underground.** Rock strata, large caverns, winding tunnels, ravines,
    underground lakes and rivers, lava lakes near the floor, aquifers and ore
    veins, plus cave Biomes chosen in three dimensions (crystal, fungal,
    dripstone, ice, lush) with their own surface features, such as stalactites.
  - **17d Biomes.** Biome choice gains altitude, distance inland and erosion, so
    ocean (warm, cold, deep, frozen), shore, river, swamp, alpine, desert-dune,
    tundra and other Biomes fall where the land suits them, and borders blend
    their ground, vegetation and colours. A Biome may shape its relief within
    its landform, which amends ADR-0022. Everything is spread by seeded, layered
    fields in metres, coarse to fine, and coarse LOD levels read the same
    fields. The Games then declare the new Biomes and water. Phase 20's bridges
    find river crossings with no change to their rule, and Meadow, whose rolling
    ground has no gap, stands its first footbridges. Weather (step 24) follows
    step 17 and reads its climate, so rain and snow fall by altitude, coast and
    Biome; weather never edits terrain.
- **Step 22** is not started. Trees and Structures exist at level 0 only
  ([ADR-0022](docs/adr/0022-biomes-are-column-content-and-a-world-pins-its-generator.md),
  [ADR-0029](docs/adr/0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md)),
  which ends at `view_distance_m` (192 m by default), so from above nothing
  stands on the ground beyond it. This step draws them at the coarse levels as
  sparse impostors from the same pure draws, since the coarse vote loses a lone
  tree, and replaces the fixed doubling of the rings with a quality curve and a
  larger default. It follows step 17 so it reads that step's fields, and its
  memory cost is measured against the LOD budget in step 6.
- **Step 27** is the largest unknown in 1.0. Machines move as a unit, so they
  need collision, lighting and meshing outside the Chunk grid, Server-owned
  motion that composes with the Player's flight and physics movement, and a save
  format for a body that straddles Chunks. Spike it before committing the scope.

### After 1.0

Not blocking 1.0, in rough order of value:

- **Network transport and multiplayer sessions.** The transport (the `Player`
  and `MovementIntent` are its first customers) plus lobbies, join and invite,
  permissions and server administration.
- **Dimensions.** More than one World layer in a session, with portals.
- **Localization and an accessibility pass.** UI scaling, colour-blind options
  and subtitles. Binding display names already carry locale keys.
- **Mod browser.** Discovery and install of Mods from inside the Game.

## Open items

- **Move the Licensor to a legal entity on incorporation.** It is currently the
  individual, which is correct and enforceable. BSL applies separately per
  version, so the switch costs nothing: name the company in the first release
  after incorporating, and older versions keep their own parameters. "Tesserae"
  itself cannot be the Licensor — it names the Licensed Work, and the Covenants
  run from a person or a company, not from a product name.
- Chunk dimensions: 32³ assumed, not measured.
- **Per-ring LOD chunk budget: measured at the default 4 km reach** (Apple M5,
  `dev` build, 1 m Voxels, a 192 m view distance, each level doubling the reach,
  so levels 0 to 5). Every level holds about as many Chunks as level 0, so the
  total grows with the number of levels and not with r². Job costs are means
  over 5 runs; the fill time is when the level has held its steady count while
  every level fills together, not alone.

  | Level | Held | Non-uniform | Meshed (non-empty) | Generate | Mesh   | Quads   | Vertex + index | Filled |
  | ----- | ---- | ----------- | ------------------ | -------- | ------ | ------- | -------------- | ------ |
  | 0     | 2167 | -           | 791 (-)            | -        | -      | -       | 41.5 MB        | 0.9 s  |
  | 1     | 2167 | 386         | 776 (221)          | 700 us   | 610 us | 135 800 | 25.0 MB        | 3.0 s  |
  | 2     | 2167 | 394         | 776 (216)          | 620 us   | 550 us | 125 800 | 23.2 MB        | 5.1 s  |
  | 3     | 2167 | 394         | 776 (216)          | 520 us   | 510 us | 119 200 | 21.9 MB        | 7.2 s  |
  | 4     | 2167 | 394         | 776 (216)          | 370 us   | 500 us | 70 900  | 13.1 MB        | 9.4 s  |
  | 5     | 1243 | 226         | 328 (73)           | 370 us   | 490 us | 17 800  | 3.3 MB         | 10.6 s |

  Vertex memory is 128 MB in all, 86 MB of it in the coarse levels. A surface
  Chunk costs 2.1 ms to generate and downsample at level 1 and 0.8 ms at level
  5, and 0.39 ms to 0.30 ms to mesh (`lod/coarse_chunk`, `lod/mesh_coarse`);
  most jobs are early-outs, which is why the means are a third of that.

  What the caps buy is a trade, not a free number. The pool is one FIFO of three
  workers and level 0's fill keeps them all busy, so coarse work that runs
  during it is level-0 work that waits. Level 0 is drawn at 1.4 s with no coarse
  levels; at the default caps (16 / 32 in flight, 16 / 16 landed) it is drawn at
  1.6 s and the distance is filled at 11 s; at 64 / 128 in flight it is 2.3 s
  and 4.5 s; at 256 / 512 it is 4.0 s and 3.4 s. The worst tick on the main
  thread stays between 0.8 and 1.2 ms in every row, so a bad cap costs fill
  latency and not frame time, as for level 0. A second pool does not remove the
  cost: it is CPU time on the same cores, and six threads instead of three bring
  level 0 back to its uncontended time and raise the worst tick from 0.95 to 1.6
  ms. There is one pool. The sweeps and their load averages are in the Outcome
  of `.scratch/2026-10-13-phase-6-lod/spec.md`.

  Level 0, from before LOD, still stands. Streaming runs on a task pool, so the
  budget is an in-flight cap and a landing cap per stage rather than a count of
  work per tick. At 1m Voxels and a 192m view distance (64 / 128 / 128 in
  flight, 64 / 256 / 32 landed) the worst tick on the main thread is 0.7 ms
  against 22-27 ms, and the load volume fills in 1.6 s against 7.2 s. Caves
  raised per-Chunk cost for the Chunks a tunnel crosses (a deep one from
  nanoseconds to about 0.7 ms across the three stages) and moved none of those
  figures: ten runs each with and without them, interleaved, give a worst tick
  of 0.6 ms and a fill of 1.58 s either way, because a ring holds one deep layer
  inside the cave band. The caps want revisiting when a Mod sets a cave floor
  deep enough to put several layers in the band; see `StreamingBudget`. Slope
  smoothing is the first terrain to pay for shaped Voxels (about +65 us to
  generate, +10-20% to mesh and +80-100% to light a surface Chunk) and moved the
  worst tick from 1.6 to 1.8 ms at load 20, so the caps did not change either.

- **Reach is configurable at load time.** Step 7 resolves the step-6 loose end
  through the Game's Declaration: full detail 32–512 m and LOD reach from that
  radius to 8192 m. Runtime changes remain deferred; tables and view parameters
  are fixed once the World opens. Terra requests 160/3000 m, below the engine's
  4096 m default LOD reach; its distant capture shows beyond the default 192 m
  full-detail radius, not beyond default LOD reach.
- **Mod host follow-ups are explicit.** No World read until a batched Snapshot
  API; no save-time tombstones or missing-Mod migration; no compile cache;
  terminal-only Trust until a pre-World window state; a flat appearance palette
  until step 9; disabled Mods recover only on reopen. Distant Mod Materials
  retain dominant-volume LOD and no emission. Closed `command` and `occlusion`
  variants require a 0.2 line to grow. The patch export-lookup question is
  resolved by the compatibility fixture, with no fallback. The per-call deadline
  thread is a future optimization; generated bindings still lift before the
  bounded post-lift check. The pre-existing `fbm2` debug overflow for seeds
  within eight of `u32::MAX` is out of scope.
- **Fluid and fire have known limits, each a later decision.** A fluid that is
  opaque and emits nothing draws its partial surfaces, and the Shapes it fills,
  unlit, because light is one value per Voxel; lava emits, and water and the
  terra's mud are translucent. Nothing climbs inside Shape geometry: water on a
  floor does not run up into a slab's or a stair's Void beside it, and a lip is
  not modelled. Level 1 is a mip-like volume vote, so a one-Voxel stream that
  shares a coarse Voxel with its floor is lost to the stone, waterlogged water
  mostly loses to its Shape, and coarse fluid is lit as air with no emission;
  levels 2 and up are generator-only, so from level 2's inner edge, 256 m at the
  defaults, held fluid and fire disappear and generated water returns. A writer
  changing the plane of light a neighbour's solve reads every tick can hold that
  neighbour's mesh back by up to four spoilt solves. Fuel beside terrain that is
  not held keeps its progress until it is, but flow beside it drops the change
  it was waiting for and starts its delay again. Reactions have no delay of
  their own, and a thin film against a cube wets all of it. Lava draining
  outdoors at 0.25 m is the costliest step measured; landing a step as one relit
  batch did not measurably cut it, so what its cost is made of is open.
- **Shaped Voxels have loose ends worth a ticket each.** Smoothing has no corner
  stairs (a corner gets a slab) and does not ease a step of two Voxels or more.
  Connection groups still belong to a Shape and not a Material after step 7, a
  connected Shape cannot be rotated, and LOD counts only its bare post. A fence
  costs 4 stairs of the cap, so 256 fit in a Chunk. Material transparency
  (glass, water, leaves) is a Material property and not a Shape one: the table,
  light dimming, occlusion and the blended render pass are done (tickets 04 and
  06 of `.scratch/2026-10-12-material-table-transparency/`), with each Chunk's
  blended quads in an exact order. A coarse blended fragment is discarded where
  the finer level has drawn its Chunk, so water is never drawn twice at a ring
  join (the LOD spec's decision 9). A Game's World may have lakes: `WaterParams`
  fills open air up to one level with a fluid the Game declared, and slope
  smoothing leaves an underwater step as a step; the engine's default World has
  none. A fluid is not solid to a body or the edit ray (`MaterialDef::solid`),
  and leaves are cut out on a pass of their own. The Client's hotbar is a
  stopgap.
- **Data-only Mods start step 10 and are the agreed next session.** A
  Declaration read from a data file beside `mod.toml`, with no component,
  through the same validation path. Most of a Game's `declare` is data, and
  since step 8.1 Terra carries more of it. A guest SDK crate was the
  alternative, and would amend
  [ADR-0004](docs/adr/0004-terra-game-in-repo-real-game-out.md)'s boundary for
  Terra.
- **Step 8.1 left these untested or unmeasured.** A trackpad's scroll adds up to
  one hotbar slot per 100 px (`PIXELS_PER_NOTCH`), a guess never tried on
  hardware. With its lakes Terra's burning row at 1 m shows at 13 s what a World
  without them shows at 12 s, and the two agree by 40 s; the gallery makes no
  difference. Generated lakes are rescanned once as their Chunks arrive, under
  the same Agenda quota as everything else, which is the likely reason the first
  fuel evaluations wait. No rule changes; a quota bounds work, not time.
- **The Server advances the Player's body.** Fixed-step flight and physics use
  core Shape collision in metres. Unknown terrain blocks physics; embedded mode
  entry is refused without relocation. Player position, mode and view persist.
  Prediction and lag compensation remain transport follow-ups. See ADR-0008.
- Forward vs deferred: measure once real scenes exist. Note it is now a real
  either/or — **SSR requires deferred, and GPU occlusion culling is documented
  as incompatible with deferred.** One or the other, not both.
- **Engine-only means nothing is playable on download.** Luanti's biggest
  adoption problem is that a new user installs it and there is no game. A
  reference game is probably not optional.

## Verified constraints (2026-10-11)

Read from pinned source tags, not rendered docs. Corrections to earlier
assumptions are marked.

### Metal ray tracing is available — **correction**

`wgpu` 30.0.1 sets `EXPERIMENTAL_RAY_QUERY` on the Metal backend
(`wgpu-hal/src/metal/adapter.rs`). The rustdoc still claims "Supported
platforms: Vulkan" and is stale. Caveats, all concrete:

- **macOS 15.0 minimum**, hard-coded, regardless of hardware; also requires
  `supportsRaytracingFromRender`.
- **Ray queries (inline) only.** RT _pipelines_ hit `unreachable!()` on Metal.
  No `vertex_return`, no TLAS binding arrays.
- M1/M2 expose the API; **hardware-accelerated** RT starts at M3 (Apple9).
- Binding pressure: without `STRICT_WEBGPU_COMPLIANCE` the budget is 29 slots
  **shared** with storage buffers, uniform buffers and vertex buffers. With it,
  RT is off entirely.
- MoltenVK ships **no** RT extensions, so Metal is the only path on macOS.
- Apple footnote: RT-in-render-pipelines and mesh shading are **mutually
  exclusive**.

### `bevy_solari` runs on Metal, but cannot eat our meshes

Hybrid, not a path tracer: rasterized deferred G-buffer, raytraced lighting on
top. Blocking requirement — `RaytracingMesh3d` demands vertex attributes exactly
{POSITION, NORMAL, UV_0, TANGENT}, `TriangleList`, `Indices::U32`, and
`MeshMaterial3d<StandardMaterial>`. Also `Msaa::Off`, and no `PointLight` /
`SpotLight` support yet. No denoiser on macOS (DLSS-RR is NVIDIA +
Windows/Linux).

### `bevy_pbr` friction points for us

- **SSGI does not exist in Bevy** — correction to an earlier claim. Indirect
  light is light probes, lightmaps, irradiance volumes, or Solari.
- **SSR requires deferred**; **occlusion culling is incompatible with
  deferred**. Pick one.
- SSAO requires `DepthPrepass` + `NormalPrepass`.
- Meshlets: Vulkan and Metal only (**not DX12**), need `TEXTURE_INT64_ATOMIC` →
  **M3+** on macOS, forbid MSAA, and force a global forward-or-deferred choice.
- 0.20 renamed/moved a lot: bloom/DOF/motion-blur → `bevy_post_process`, AA →
  `bevy_anti_alias`, light types → `bevy_light`. Shaders are **WESL**, not WGSL.

### LOD technique, from Distant Horizons and Voxy

Both reach extreme range **without raymarching** — Voxy's voxel-brick + mip
pyramid + screen-space error ends in rasterized greedy quads, which validates
our plan. DH hits 4096 chunks on GL 3.3-class rasterization of column-extruded
boxes, but its LOD primitive is a heightfield column, so overhangs and caves
degrade — we must not copy that primitive.

The technique to copy: **render LOD levels into a separate target with their own
depth, then composite.** Neither shares the near-field depth buffer at extreme
range. That is how both avoid z-fighting.

**Bevy 0.20 does not need it for precision.** Its projection is infinite
reverse-Z (`perspective_infinite_reverse`; `far` only places the frustum's
culling plane) with an f32 depth buffer, so depth at 4 km is not the problem the
technique was invented for on GL-class depth. Cameras that share a
`RenderTarget` share its colour and depth textures, and a later camera with
`ClearColorConfig::None` draws over an earlier one after its own depth clear, so
a camera per level is the technique at no extra texture. What it is for here is
**coverage priority**: a pixel a finer level drew shows that level whatever the
coarse depth. A single shared far camera with a depth push toward the viewer was
tried first and cannot work, because a coarse surface up to one coarse Voxel
above the finer one is `dh / sin(angle)` further along the ray, far more than
any push at a grazing view.
