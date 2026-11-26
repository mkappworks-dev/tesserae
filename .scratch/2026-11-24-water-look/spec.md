# Water that looks natural

Status: ready-for-agent

Source: branch `feat/water-look`, cut from `main` at `0b03c594`. Decisions
settled in a grilling round on 2026-11-24 and 2026-11-25. Tickets:
[issues/](issues/). Decisions:
[ADR-0038](../../docs/adr/0038-water-motion-is-drawn-never-simulated.md). Builds
on `.scratch/2026-11-16-phase-17-terrain-and-water/` (rivers, lakes, the
drainage solve) and on `.scratch/2026-11-23-colour-art-and-shader/`, merged to
`main` as #75.

## Problem Statement

Asked on 2026-11-24: water should look natural, flowing from mountains to rivers
to lakes or the sea, with waves on the coast and the open sea and rivers that
run.

What the code does today, read from source:

- **Milky.** Every water surface is one flat colour at 65 % alpha, whatever its
  depth. A 30 m sea and a 20 cm puddle both let a third of the floor through, so
  deep water reads as a blue film over sand. The colour spec names this and
  leaves it out of its scope.
- **Matte.** Water shares the blended material with glass, which copies the
  opaque terrain's roughness 0.95 and Bevy's default reflectance. There is no
  sun glint and no sky reflection; at a grazing angle water shows a faint grey
  sheen.
- **Static.** No render shader reads time. Nothing ripples, runs or rolls.
- **Tiled.** Terra's water is a 16 by 16 tile of stripes repeated every Voxel
  across a merged lake top, a lattice at distance.
- **Rivers do not run.** A generated river is a staircase of still Source pools
  held by one-Voxel rock lips. Nothing falls over a lip and nothing shows which
  way the river goes.
- **Rivers start too late and end abruptly.** A stream begins only once about
  0.12 km² drains into it, so mountain slopes are dry and rivers appear
  mid-valley. Where one reaches the sea it simply meets it: no estuary and no
  delta, though step 17's rivers ticket promised deltas. Every river bends by
  the same 14 m warp, so a 40 m river wiggles like a brook.
- **Underwater is one hue.** The fluid fog fades everything to the palette
  colour at one rate, so red does not vanish first as it does in real water.

## Solution

Water gets a physically based look inside the existing chunk material, drawn
motion that the Server works out and the Client animates, and a fuller river
network from the Generator.

1. **Fixtures.** Lake, river, coast and underwater capture and profiling
   fixtures, and a baseline taken before anything changes.
2. **Hydrology.** Headwater brooks on high ground, meanders that grow with the
   river, and river mouths: estuaries, and deltas for large rivers. A change to
   Generator version 3 in place, before the first release.
3. **Flow and Falls.** The Generator answers each column's Flow (direction and
   speed in m/s) and where each Fall is. Simulated Fluid gets the same answers
   from its surfaces.
4. **Water columns.** The Server's mesh job ships per-column water data with
   each wet Chunk's mesh: depth, Flow, foam and Fall flags.
5. **The look.** Near-mirror roughness and water's real reflectance (F0 0.02),
   Fresnel-weighted sky reflection and sun glint, ripples in time, colour and
   opacity from light absorbed over the water's depth, shore foam, running
   rivers, Falls drawn as sheets and foam, and sea waves along a World's Wind.
   Underwater, each colour fades at its own rate.
6. **Declared look.** A Game or Mod may declare a Material's water look
   (absorption, scatter tint, roughness, foam colour); the engine's pure-water
   values are the default.

Motion is drawn, never simulated
([ADR-0038](../../docs/adr/0038-water-motion-is-drawn-never-simulated.md)):
nothing carries a body and generated rivers stay still pools.

## User Stories

1. As a player, I want a deep lake to look darker and bluer than its shallows,
   which show the sand, so that I can read depth at a glance.
2. As a player, I want water to mirror the sky at a low angle and let me see
   into it when I look down, so that it reads as water and not as tinted glass.
3. As a player, I want the sun to glint on water, so that a coast in the
   afternoon looks alive.
4. As a player, I want a still lake to ripple gently, so that it does not look
   frozen.
5. As a player, I want a river's surface to run downstream, faster where it is
   steep, so that I can tell which way it flows without a map.
6. As a player, I want white water where a river is steep, so that rapids look
   like rapids.
7. As a player, I want water to pour over each step of a river into the pool
   below, foaming where it lands, so that a mountain river looks like falls and
   not like stairs.
8. As a player, I want waves rolling in from the open sea and breaking as foam
   on the beach, so that the coast feels like a coast.
9. As a player, I want waves all to come from one direction, the World's Wind,
   so that the sea looks like one sea.
10. As a player, I want a foam line where water meets land, so that the
    one-Voxel step at the shore is softened.
11. As a player, I want streams to start high on mountain slopes and join into
    rivers, so that water visibly comes from the mountains.
12. As a player, I want large rivers to swing in wide bends and brooks in tight
    ones, so that rivers look their size.
13. As a player, I want a river to widen into an estuary or split into a delta
    where it meets the sea, so that its end looks natural.
14. As a player, I want the colour to fade with distance under water, red first
    and blue last, so that diving feels like being under water.
15. As a player, I want water at a distance to look like the same water as near
    me, without a colour step where the detailed ring ends, so that the far
    field reads as one World.
16. As a player, I want water at a distance not to sparkle or crawl, so that the
    shimmer fix of #74 holds for water too.
17. As a player, I want the river I swim in to feel like a lake, with nothing
    pushing me, so that swimming is predictable (ADR-0038).
18. As a player who pours water or dams a river, I want my water to run and look
    like the World's water, so that what I build matches what was generated.
19. As a player in a Settlement, I want its waterfall to look like a waterfall,
    so that it matches the generated ones.
20. As a player on a weaker machine, I want water to cost little frame time, so
    that my frame rate holds over a lake.
21. As a Game author, I want my water to look physically right without declaring
    anything, so that I get good water for free.
22. As a Game author, I want to declare my water's absorption, scatter tint,
    roughness and foam colour, so that a swamp can be green and murky and a
    lagoon clear.
23. As a Mod author, I want the `waters` Mod's water to look like pure water, so
    that every Game that uses it starts from a natural look.
24. As a future Look pack author, I want water's normal and foam textures to be
    engine images I can later replace, so that a pack can restyle water without
    engine changes.
25. As a developer, I want Flow to be a pure function of the seed and metres,
    and a Fall of the seed, metres and Voxel size like the lips, so that both
    agree in every Chunk order.
26. As a developer, I want water's ripple, flow and wave clocks to repeat
    exactly within Bevy's hourly time wrap, so that the surface never jumps.
27. As a developer, I want lake, river, coast and underwater fixtures, so that
    every water change is judged against the same views and frame times.
28. As a developer, I want the World's water answers (Water surface, Flow, Fall)
    to come only from the Server, so that the Client reads no World storage
    (ADR-0002).
29. As a reviewer, I want each look change shown as before and after captures
    from fixed Eyes, so that I approve looks, not code.

## Implementation Decisions

### Settled in the grilling round

- **Physically based defaults.** Pure water's absorption, measured by Pope and
  Fry: about 0.0092 m⁻¹ at 450 nm, 0.0565 at 550 and 0.34 at 650 for blue, green
  and red. Reflectance F0 0.02, from an index of refraction of 1.33 (Bevy's
  `reflectance` about 0.35). A stylised look is a Game's declared look, never
  the default.
- **Motion is drawn (ADR-0038).** Flow, Falls and waves are presentation: the
  Server works them out per column when it builds a mesh and the Client animates
  them on its own clock. Waves never move the drawn surface more than 0.15 m
  from the World's, the underwater test's hysteresis; this spec draws waves in
  the shading only, so the surface stays flat. Generated rivers stay still pools
  of Sources; a Fall is a drawn sheet and foam.
- **Vocabulary.** **Flow**, **Fall** and **Wind** are glossary terms. The
  per-Chunk render data is "water columns" in specs and code, and is not a
  glossary term. "Water table" is not used for it: the glossary's **Aquifer**
  uses that phrase for ground water.
- **Order.** Hydrology, Flow and Falls, and water columns touch only
  `tesserae-core` and the Server, and come first. The tickets that touch
  shaders, fog, the Client's chunk mesh upload, Material textures, the WIT
  `appearance` record or Terra's water build on the colour work, which this
  branch was rebased onto before any code (ticket 07). If the first release
  comes close, the hydrology tickets split into their own pull request.
- **Both looks here.** Engine defaults and the declared look both land in this
  spec. The declared look joins contract line 0.3 in place, as every feature
  before the first release has; no 0.4 line.
- **Scope.** In: the water look in the shader; underwater light that fades by
  colour; water columns; running rivers; drawn Falls; the declared look; sea and
  lake waves in the shading; and the hydrology. Out: sloped tops for Flowing
  fluid, caustics, a displaced sea surface, refraction and erosion. The shore
  keeps its Voxel step, softened by a foam band.
- **Wind.** One Wind per World, a direction and strength drawn from the World
  seed, constant for the World. The Server publishes it to the Client; Weather
  (step 24) takes it over. Not a Biome declaration: waves would turn at every
  Biome edge on a coast.
- **Budget.** On the M5 at 1080p offscreen, each of the lake, river, coast and
  underwater fixtures costs at most +0.5 ms median frame against `main`;
  `flight` is no worse than `main` beyond noise.

### The water path

- Water is drawn by the existing chunk material (`StandardMaterial` with the
  chunk extension) in the existing blended layer, so it keeps sun shadows on its
  surface, clustered lights, distance fog and tonemapping. No second renderer.
  Water and glass stay in one blended layer: splitting them would let Bevy
  misorder a large lake against a small pane, and only refraction would need it.
- **Which Materials take it.** The Server resolves each Material's water look
  and publishes them to the Client in a content message beside
  `MaterialPalette`, because tags, Fluid rows and render classes never reach the
  Client. A blended Fluid Material tagged `water` gets the engine defaults; from
  ticket 15, a blended Fluid Material with a declared water look gets that.
- **Per-slot table.** The Client fills a per-slot uniform table from that
  message: absorption per channel, scatter tint, roughness, foam colour. A slot
  that is not water holds an explicit not-water flag and draws as today: glass,
  steam. Not a face-word bit: the colour branch takes bits 30 and 31. 257 slots
  of three vec4s is about 12 KB, inside a uniform's limit.
- **Blending.** The blended material moves from `AlphaMode::Blend` to
  `AlphaMode::Premultiplied`, a pipeline key change. Under it Bevy blends
  premultiplied colour and leaves the shader's output alone, so the chunk shader
  premultiplies every blended fragment itself. Glass and steam then draw exactly
  as before, and the water path writes its body colour times its alpha plus its
  reflection, so reflection is not faded by alpha.
- **Surface.** Roughness about 0.08, tuned on captures; reflectance for F0 0.02;
  the normal perturbed by ripples. The water path skips the colour branch's
  ambient occlusion: water is not in the depth prepass, so it would take the
  floor's occlusion. Water at level 0 no longer samples the Material's 16 by 16
  tile; the tile stays for items, the inventory and coarse colour.
- **Sky reflection.** On the colour branch the sky is its own material, with a
  zenith colour and a sun direction, mixed after tonemapping. Ticket 08 gives
  the chunk extension the same zenith colour and sun direction, kept in step
  with the sky's, and factors a shared WESL function that returns the sky colour
  along a ray before tonemapping. The reflection is that colour weighted by
  Fresnel, plus the sun's specular from Bevy's directional light. No
  screen-space reflection (Bevy's needs deferred rendering, which the chunk
  material does not implement) and no environment map.
- **Seen from below.** Water is double-sided. Seen from under the surface, the
  water path draws no sky reflection; the underwater fog does the rest.
- **Colour from depth.** The view path through the water is the column's depth
  over the cosine of the view angle. Each channel keeps
  `exp(-absorption × path)` of the floor behind; what is lost is replaced by the
  scatter tint. Alpha follows the mean transmittance, so shallows are clear and
  deep water darker and bluer. With pure water's absorption, 20 m straight down
  still lets through most of the blue: deep water is fully opaque only with a
  Game's own absorption.
- **Time.** The shader reads Bevy's `globals.time`, which wraps every hour.
  Every ripple, flow-phase and wave frequency is defined in Rust, passed to the
  shader as shader defs, and is a whole multiple of 2π/3600 s, so the surface
  repeats exactly at the wrap and never jumps.
- **Distance.** Ripple and wave normals fade with distance, and roughness rises
  as normal detail is minified, so low roughness does not sparkle. UV
  derivatives are taken at the top of the fragment stage, before any discard, as
  #74 does.
- **Shore foam.** Foam where a column's depth falls below a few tenths of a
  metre, broken up by the foam mask and moving with Flow.
- **Engine textures.** A tileable normal map (about 256²) and a foam mask (about
  128²), generated by engine code at start-up, with no third-party art. They are
  engine images a Look pack could later replace; Look pack loading is not built
  here.

### Water columns

- A wet Chunk's mesh carries one entry per water column of the Chunk plus a
  one-column apron, about 4 bytes each: Flow along x and z, depth in metres
  (quantised), and foam and Fall flags. A dry Chunk carries none. About 4.6 KB
  for a wet Chunk.
- Built by a `tesserae-core` function that takes the lit Chunk, its neighbours
  and a trait of water answers that `Worldgen` implements; the Server's mesh job
  calls it, and so gains the Generator as an input. Each column is classified on
  its own:
  - **Generated:** its top water Voxel is a Source of the World's water at the
    Voxel the Generator's Water surface tops out in. It takes the Generator's
    Flow and Fall answers, and depth from that drawn top down to the generated
    ground.
  - **Every other column** (poured, simulated, a dammed river, a Settlement
    pond): Flow down the surface gradient between neighbours; a column with
    descent, or with Fluid above it, is falling; depth from a scan down the
    column as far as the job sees, saturating at a depth past which red and
    green are gone.
  - **Apron corners:** the job holds only the six face neighbours. Diagonal
    apron columns take the Generator's answers where generated, and otherwise
    repeat the nearest held column.
- They travel inside `ChunkMesh` with the blended layer and are uploaded into
  the layer's range of the engine's storage buffer, after its records, with the
  header naming where they start. ADR-0034's one-record-per-quad stays as it is.
- A merged quad spans many columns, so the fragment stage finds its column from
  the quad's first Voxel plus the floor of its UV, as the colour branch's
  per-Voxel tile turn does. The vertex stage forwards only the layer's buffer
  offset. Flow and depth are filtered across neighbouring columns.

### Flow and Falls from the Generator

- **Flow** on the drainage, a pure function of the seed and metres: for the
  nearest stream segment in reach, the segment's direction turned by the local
  slope of the meander warp so it follows the bends, and a speed from the
  segment's level drop over its length and its catchment, clamped to a natural
  range (a slow lake drift to a few m/s in rapids). Lakes drift slowly toward
  their outlet, faster near the spill. The sea has no Flow: its motion is waves
  from Wind.
- **Falls** follow the lips, which are asked on each Voxel size's own lattice,
  so a Fall is a pure function of the seed, metres and Voxel size. A lip is a
  dry column whose ground was raised to hold a higher pool beside it. A Fall
  runs from that pool over the lip to the next column past it, when that column
  has a lower Water surface. A lip with dry ground past it has no Fall.
- New Generator code uses no platform maths (ADR-0032). Neither answer changes
  terrain, so neither moves a digest.

### Falls drawn

- The mesher emits a thin blended sheet of the upper pool's water Material on
  the lip's far face, a sixteenth off the rock so it does not fight it for
  depth, from the upper Water surface to the lower one, and flags the lower
  pool's columns beside it for foam. Vertical faces of a water Material draw
  falling Flow: the same look serves generated sheets and simulated falling
  Fluid, such as a Settlement's waterfall.
- A sheet is not in the plane of a lake's top, so every Chunk with a lip gets
  its blended layer sorted. The sort is capped by its frame budget; what is
  measured is how many frames a mesh waits for it.

### Sea waves

- Four to eight Gerstner waves around the Wind's direction, in the normals only.
  Their steepnesses sum to at most 1, so crests never loop.
- Each wave's angular frequency is fixed and quantised to the clock. Its
  wavenumber is solved per fragment from ω² = g k tanh(k D) over the filtered
  depth, so waves shorten in the shallows without tearing the phase where depth
  changes. Each wave's direction bends toward the shore along the depth's
  gradient. Waves too short for a pixel's footprint are dropped.
- A shore band where depth is small: a travelling breaker phase driven by depth,
  steepening and foaming toward the beach.
- Lakes get ripples scaled by the Wind's strength, much smaller than the sea's
  waves.

### Wind

- Direction and strength drawn from the World seed on the Server, published to
  the Client when a World session starts. Constant until Weather owns it at
  step 24.

### Underwater

- For a Fluid on the water path, the underwater fog uses Bevy's atmospheric
  falloff: extinction per channel from its absorption, in-scattering from its
  scatter tint. Its range comes from the extinction, not from dimming, and the
  clear colour is the in-scattered colour seen at full extinction. The colour
  branch's haze does not apply under water. Other fluids (lava) keep today's
  single-hue fog.

### Declared water look

- An optional water look per Material, declared beside `appearance` in contract
  line 0.3: absorption per channel in m⁻¹, scatter tint, roughness, foam colour,
  each optional, with defaults filling the rest. Data-only Mods declare it in
  TOML beside `[[appearance]]`. Bounds are validated like appearance, and a row
  naming a Material that is not a blended Fluid is refused with the Mod and row
  named. The WIT copies in the starters and the contract, authoring and data-Mod
  docs follow, as they did for the colour branch's appearance change.
- Terra declares a look tuned on captures for its water. `waters` declares pure
  water, which is what Ember, Lumen, Meadow and Tidewater draw, since their
  water is `waters:water`.
- A Look pack may replace a declared water look as it replaces appearance.

### Hydrology

- **Headwater brooks.** Streams start at a smaller catchment than today's 120
  drainage lattice points (0.12 km²), as brooks that widen downstream into
  today's widths. The valley's dig scales with width too, or a narrow brook
  still draws a wide wet band. The per-column cost (the drainage lookup is most
  of the heightfield's cost) is measured on the `worldgen_v3` bench; a
  regression over 5 % is brought to the user before it lands.
- **Meanders grow with the river.** The warp's amplitude and period scale with
  river width. That means a warp per segment rather than one per point, so the
  cost is measured under the same 5 % gate, and the amplitude stays inside the
  valley's reach so it still fits the drainage stencil.
- **River mouths.** Where a stream ends at the sea, its channel widens into an
  estuary; above a width, it fans into a few distributaries over a shallow
  shelf: a delta.
- New Generator code uses no platform maths (ADR-0032).
- Each changes version 3's output in place before the first release, and the
  recorded digests are recorded again, as the pre-release amendment to
  ADR-0033 allows.
- Settlements, bridges, coarse levels and spawn read the Water surface, so they
  follow the new network unchanged.
- Generated water still never has open air beside it: the lips stay.

### Coarse levels

- Coarse water takes the same water path, calm, with no Flow. Level 1's columns
  follow level 0's per-column rule, since the coarse mesh job already holds the
  Generator; levels 2 and up take the Generator's depth. The colour at the edge
  of level 0 matches level 1's, with no step.

## Testing Decisions

A good test checks what the World answers or what the mesh contains, not how it
was computed. Seams, highest first:

- **The Generator's water answers** (`tesserae-core`, beside the terrain and
  drainage tests):
  - Flow points downstream along every stream (positive along the segment's
    descent), is zero on the sea, and is a faint drift toward the outlet on a
    lake.
  - A Fall is answered across every lip that has a lower pool past it, and
    nowhere else; a lip beside dry ground has none.
  - Flow is equal in any Chunk order and with a cold cache, and the same in
    metres at 1 m and 0.5 m Voxels, like the existing river and lake tests.
    Falls are equal in any Chunk order and match each Voxel size's own lips.
  - Hydrology: streams begin within a set distance of high ridges; meander
    amplitude grows with width; every river that reaches the sea ends in a mouth
    wider than its channel. The existing drainage tests keep passing, as does
    "generated water has no air below or beside it". Digests are recorded again
    only in the hydrology tickets; every other ticket leaves them bit-identical.
- **The water columns** (`tesserae-core`, through the column builder with a test
  double for the water answers, beside the meshing tests):
  - A dry Chunk carries none. A generated lake's depth runs from its drawn top
    to its ground.
  - Simulated flow points down the surface gradient; falling Fluid is flagged; a
    dammed river column and a Settlement pond take the scan rule.
  - A lip with a lower pool past it emits one sheet spanning the two surfaces,
    and the pool below is flagged for foam.
  - Every existing fluid meshing test keeps passing unchanged.
- **The Client**:
  - The layer header names where the water columns start, and they upload whole
    (beside the chunk mesh header tests).
  - The blended material is premultiplied, replacing the test that pins
    `AlphaMode::Blend`.
  - Every ripple, flow and wave frequency is a whole multiple of 2π/3600 s (a
    unit test on the Rust constants the shader defs come from).
  - The underwater tests gain per-channel extinction for water; lava keeps
    today's look.
  - The water-look message reaches the Client and fills the per-slot table, as
    `MaterialPalette` does.
- **Declarations** (the `tesserae-mods` data tests and their full fixture): a
  data water look decodes to what a component returns; bad values and non-fluid
  rows are refused naming the Mod and field.
- **Captures**: `TESSERAE_SCREENSHOT_OFFSCREEN=1`, from the fixtures' fixed
  Eyes, before and after every look ticket, reviewed by the user. One `/tmp`
  folder, never committed. Eyes sit off whole metres so they do not land on a
  Voxel plane. "A moving pair" is two captures a small step apart, scored as in
  `.scratch/2026-11-23-texture-shimmer/`.
- **Budget**: `scripts/profile-frames` with the method of
  `.scratch/2026-11-17-render-packed-quads/`: release `profiling` builds,
  interleaved rounds on a quiet host, run in mirrored order. The A side is
  `main` with ticket 01's fixture commit cherry-picked. Run at 01 and 16, and at
  any ticket that adds GPU work if its captures suggest a cost.
- Demo Game tests run on Terra only.

## Out of Scope

- Sloped tops for Flowing fluid (would amend ADR-0034).
- Caustics on the floor under water.
- A displaced sea surface (vertex waves) and the subdivided sea top it needs.
- Refraction and a Quality setting for it.
- Erosion that carves valleys by discharge; after the first release it needs
  Generator version 4.
- FFT oceans, screen-space reflections, a depth prepass and environment maps.
- Tides, anything that carries bodies, boats.
- Loading Look packs.
- Wind from Biomes or Weather.
- Rivers and inland lakes at coarse levels standing at their level-0 height:
  coarse levels snap only the sea today, and that stays.
- Glass seen through water, and water through glass, beyond what the blend sort
  already orders.

## Further Notes

- Findings from the research, in our own words:
  - Flow-mapped normals distort a tiled normal map along a per-point flow
    vector. The distortion holds for about a third of the map's period, so two
    layers half a period apart are cross-faded, with a noise offset per point to
    hide the pulse. Normal strength scales with speed. One flow vector per Voxel
    column is about the density shipped games used.
  - Over finite depth, wave frequency follows ω² = g k tanh(k D). Quantising
    every frequency to the clock's period makes the animation loop exactly.
  - Water's reflectance at normal incidence is about 2 %; Schlick's
    approximation of Fresnel is already in Bevy's lighting.
  - The drainage solve is a priority flood with ε for flats, D8 directions and
    flow accumulation in reverse flood order. Width already grows with the
    square root of catchment, as river hydraulic geometry does.
- Measured so far: the median `flight` frame is 5.6 ms on the M5 at 1080p
  offscreen (DESIGN.md). Water's real cost is fragment work times overdraw, and
  water rarely overdraws itself.
- DESIGN.md's step 17 and render notes are updated when the branch lands.
