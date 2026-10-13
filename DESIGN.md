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

| Decision             | Choice                                                                                                             | Why                                                                                                                                                                                                                                                                                                             |
| -------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Renderer             | CPU greedy meshing → rasterized triangles                                                                          | Large streamed worlds, 15 years of prior art. Not raymarched/SVO.                                                                                                                                                                                                                                               |
| Voxel size           | 1m default, per-world parameter                                                                                    | See "The voxel-size rule".                                                                                                                                                                                                                                                                                      |
| Non-cube shapes      | **In scope** — stairs, slabs, fences, Luanti-style nodeboxes                                                       | Parity with Luanti's strongest feature. Expensive; see "Shaped voxels".                                                                                                                                                                                                                                         |
| Engine               | Bevy 0.20                                                                                                          | Not 0.19 — superseded 2026-10-08.                                                                                                                                                                                                                                                                               |
| Bevy depth           | Full `bevy_pbr`, chunk materials via `MaterialExtension`                                                           | Shadows, SSAO, TAA, bloom, tonemapping, atmosphere, volumetrics come free and maintained. Hand-writing a renderer is a 12–24 month detour no user can see. Ethertum (543★, most active Bevy voxel game) does exactly this.                                                                                      |
| Material model       | **PBR**                                                                                                            | Physical parameters + energy-conserving BRDF, so materials look right under any lighting. Minecraft vanilla is flat albedo × baked light — the reason it looks like 2011, and why LabPBR packs exist to retrofit PBR.                                                                                           |
| Forward vs deferred  | **Undecided — measure it.** Bevy's default is forward + depth prepass + clustered lighting; a deferred path exists | Clustered forward handles a few hundred lights. Deferred decouples light count from geometry, which suits voxels, but costs MSAA and still needs a forward transparency pass. Not a day-1 commitment.                                                                                                           |
| Graphics API         | wgpu → Metal / Vulkan / DX12                                                                                       | OpenGL is frozen at 4.1 on macOS (2010 spec, deprecated 2018, **no compute shaders**) and macOS is platform #1. Cost of wgpu: validation overhead and no backend-specific extras such as Metal RT.                                                                                                              |
| View distance        | 4km                                                                                                                |                                                                                                                                                                                                                                                                                                                 |
| LOD                  | Mip chain baked into the chunk format from day one                                                                 | Retrofitting is a storage-format migration plus a re-mesh of every saved world. Distant Horizons needs a separate datastore only because Minecraft's format has no LOD; we own the format.                                                                                                                      |
| Multiplayer          | **Client-server architecture from day 1**, server runs in-process for singleplayer. Network transport deferred     | Minecraft Java's integrated-server model. "Add multiplayer later" is the most expensive retrofit in game dev because singleplayer code mutates world state directly. Build the split now, ship the transport later.                                                                                             |
| Modding              | WASM — `wasmtime`, component model, WIT interfaces                                                                 | No stable Rust ABI; dylib mods break on every rustc release, cannot be sandboxed, and force per-OS-per-arch builds onto modders. Lua would cap performance, which is the differentiator. Mods can still be written in Rust → `wasm32-wasip2`.                                                                   |
| Code licence         | BSL 1.1, 4-year per-release Change Date, Change License Apache-2.0                                                 | **Verified valid.** BSL Covenant 1 requires compatibility with "GPL Version 2.0 **or a later version**"; Apache-2.0 is GPLv3-compatible, so it qualifies. CockroachDB 22.2 and Sentry 23.1 both shipped BSL 1.1 + Apache-2.0. The "Apache is GPLv2-incompatible" objection comes from mis-quoting the covenant. |
| Additional Use Grant | Unlimited use of the engine; restriction applies only to forks of the engine itself                                | Anything narrower bans the audience an engine exists for. Name the restriction explicitly in the grant text.                                                                                                                                                                                                    |
| Media licence        | **CC0** for default and placeholder assets (CC BY 4.0 if attribution is wanted)                                    | **Not CC BY-SA.** Sharealike is viral on derived art, so a studio with proprietary art cannot use our textures as a base — friction exactly where an engine wants none. Brand protection is a **trademark on "Tesserae"**, not the asset licence.                                                               |

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
- **`tesserae-mods`** — WASM host, WIT interface definitions, the batched
  command queue. Deps: `wasmtime`. No Bevy.
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

Cost of small voxels is non-linear: 0.25m is 64× the voxels per cubic metre, so
an unchanged chunk budget covers a quarter of the distance. View distance in
metres scales as 1/`voxel_size`.

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

Geometric rings. LOD0 covers the configured full-detail radius; each level
doubles its span, so ~8 levels reach 4km with 32m chunks and total loaded chunk
count grows with the number of levels rather than with r². Level N+1 aggregates
2×2×2 voxels of level N by dominant material, then greedy-meshes normally — the
mesher does not need to know which level it is meshing.

Per-ring chunk budget is a number to measure, not to guess.

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
- post-process settings: bloom threshold, tonemapping curve, fog parameters,
  SSAO strength
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
- `games/demo/` and `assets/` are deliberately **not** BSL — see ADR-0004.

## Build order

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
6. LOD mip chain out to 4km.
7. `tesserae-mods`: WASM host, WIT interfaces, batched command queue.
8. Fluid simulation.
9. Look-pack API, water shading, visual polish.
10. Network transport. The `Player` and `PoseReport` are its first customers.

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

- **Shaped Voxels have loose ends worth a ticket each.** Smoothing has no corner
  stairs (a corner gets a slab) and does not ease a step of two Voxels or more.
  Connection groups belong to a Shape and not a Material until step 7, a
  connected Shape cannot be rotated, and LOD counts only its bare post. A fence
  costs 4 stairs of the cap, so 256 fit in a Chunk. Material transparency
  (glass, water, leaves) is a Material property and not a Shape one: the table,
  light dimming and occlusion are done, and the blended render pass is ticket 04
  of `.scratch/2026-10-12-material-table-transparency/`. The Client's hotbar is
  a stopgap.
- **The Player's pose is rate-limited, not true.** The Server clamps a pose that
  moves faster than `PlayerLimits` and tells the Client, but flying is the only
  way to move, so a modified Client can still fly at the cap and edit within
  reach of where it goes. A controller (gravity, collision, a walking speed),
  tolerances derived from it, client prediction, and lag compensation (which
  needs the transport's clock) are the follow-up. See ADR-0008 and
  `.scratch/2026-10-12-server-edit-origin/issues/05-*`.
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
