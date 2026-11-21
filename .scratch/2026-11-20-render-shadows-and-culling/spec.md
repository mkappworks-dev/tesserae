# Shadows and culling: the GPU cost of drawing Chunks

Status: ready-for-agent. Decisions Q1 to Q18 were settled in a grilling round on
2026-11-20.

Source: branch `tech/render-shadows-and-culling`, cut from `main` at `5285969a`.
Builds on `.scratch/2026-11-17-render-packed-quads/` (tickets 01, 06 and 07 hold
the gate method), `.scratch/2026-11-01-gpu-pixel-cost/` and
`.scratch/2026-10-29-streaming-performance/` (tickets 10 and 12 tried occlusion
culling and the visibility cull). Tickets: [issues/](issues/).

## Problem Statement

The frame is GPU-bound: about 4.2 ms of GPU against 0.7 ms of main CPU at 1080p
offscreen. Since ADR-0034 the vertex shader reads every quad from the quad
buffer, so vertex work is the cost of a Chunk pass, and the sun's four cascades
run it again for every level-0 Chunk they hold, every frame.

What the code and Bevy 0.20.1 do today, read from source:

- One `DirectionalLight` with Bevy's default cascades: four, with far bounds 10,
  25, 61 and 150 m, overlap 0.2, a 2048 map and Gaussian filtering. Every
  cascade is refitted to the camera's view slice, snapped to texels, cleared and
  redrawn every frame; Bevy keeps no shadow map between frames.
- The sun never moves; the engine has no day cycle yet (DESIGN.md step 9).
- Only level 0 casts: the sun's render layers are level 0's, and coarse levels
  do not exist inside level 0's ring (ADR-0019). Opaque and cutout layers cast,
  blended ones do not; avatars and drops cast too.
- On Metal every mesh asset is its own indirect draw in every pass and cascade,
  culled or not, since wgpu has no multi-draw count there. Each Chunk layer has
  its own placeholder mesh, so each is a draw in every view it is listed in.
  Blended layers are a sorted phase, drawn one by one anyway.
- wgpu's indirect-call validation runs in release: Bevy drops it only when the
  configured backends lack DX12, and its default backends include DX12 even on
  macOS. It adds a compute pass, a staging buffer and a copy before every
  indirect render pass.
- The profiler's markers misattribute the shadows. The pass that draws them is
  ordered before `prepass_end`, inside the 0.25 to 0.33 ms span earlier tickets
  called the depth prepass; the camera has no depth prepass. The 0.51 to 0.77 ms
  called shadows is probably mostly compute between the passes; ticket 01 splits
  it.

## Solution

Tickets in this order (Q18), each gated alone:

1. **Markers and baseline.** A marker after Bevy's early shadow pass, the
   comment in `profiling/gpu.rs` corrected, and the baseline on `origin/main`:
   timing, GPU, and a `no-shadows` series for the shadows' total.
2. **Indirect validation off** in release on macOS.
3. **1a: no cutout casters in the far cascades.**
4. **1d: a shadow-only vertex path.**
5. **Item 4: shared placeholders** in size buckets.
6. **1b: cascade tuning**, chosen on captures.
7. **1c: cascade caching** of the far cascades.

Item 2 (GPU occlusion culling) and item 3 (underground visibility culling) are
`wontfix`; see below.

## User Stories

1. As a player, I want sun shadows to cost less GPU time, so that the frame rate
   holds at higher resolutions.
2. As a player, I want nearby shadows, mine and other players' included, exactly
   as they are, so that the saving is not where I look.
3. As a player, I want every change to distant shadows shown to the developer on
   captures and approved before it ships, so that quality is never traded
   silently.
4. As a player, I want shadows to follow terrain as it streams in within a frame
   or two, so that caching shows no stale ground.
5. As the developer, I want the GPU markers to name the shadow pass, so that I
   optimize the pass that costs.
6. As the developer, I want each item gated alone against the commit before it,
   with `origin/main` in every series, so that a losing one is dropped without
   the others.
7. As the developer, I want the reasons occlusion culling and the visibility
   cull stay out recorded with their numbers, so that nobody retries them blind.
8. As the maintainer, I want the places this work leans on Bevy internals
   guarded and listed, so that a Bevy upgrade that moves them is caught.
9. As a Mod author, I want nothing in the WIT contract or my Declarations to
   change, so that this work is invisible to Mods.

## Implementation Decisions

- **Markers first (Q9).** Every later gate reads the corrected spans. The
  `no-shadows` switch gives the shadows' whole cost where Apple's
  encoder-boundary timestamps blur passes together.
- **Indirect validation off (Q10).** On macOS the backends are
  `Backends::from_env().unwrap_or(Backends::METAL)` and release builds drop
  `VALIDATION_INDIRECT_CALL`, which Bevy decides inside `WgpuSettings::default`
  from its own backends, so setting the backends alone keeps it; debug builds
  and `WGPU_VALIDATION_INDIRECT_CALL=1` keep it.
  Bevy writes every indirect argument itself, so undefined behaviour from a
  malformed one is not a new risk.
- **Item 2 is `wontfix` (Q11).** Bevy's two-phase culling adds a full depth
  prepass of every LOD level through the vertex shader, the very cost being cut,
  issues every draw three times on Metal whether culled or not, and gives shadow
  cascades no late phase. That fits streaming ticket 10's 4.7 to 17.7 ms, though
  the source alone does not account for all of it. Reopen only after item 4 has
  collapsed the draws, as a measured spike.
- **Item 3 is `wontfix` (Q1).** Streaming ticket 12 built the cull for drawing,
  kept culled Chunks casting so a hill behind a ridge keeps its shadow, and
  measured no effect (`cave` 3.62 against 3.60 ms). It cannot touch the shadow
  passes, and the camera spans it could cut are about 0.4 ms. Reopen when a
  fixture shows it removing more than 30% of drawn quads.
- **Far means cascades 2 and 3 (Q16)**, from 25 m at the defaults; revisited if
  1b changes the count or the bounds. 1b chose three cascades, 10, 61 and 150 m
  (2026-11-21): far is now cascade 2, from 61 m, and 1c measures whether
  caching cascade 1 (10 to 61 m) as well pays.
- **1a: no cutout in the last cascade.** A system after Bevy's caster
  visibility removes cutout layers from cascade 3's visible list, from 61 m.
  Cutout is the caster that runs a fragment shader in the shadow pass. Built
  first for cascades 2 and 3, from 25 m; chosen on captures (2026-11-21) for
  61 m, which kept 75 to 99% of the saving and shaded trees seen in flight.
- **1d: shadow-only vertex path (Q12).** The extension's `specialize` sees the
  directional shadow key and adds a shader def; the vertex shader then computes
  the position only, skipping the (Material, light) table read, the UVs and the
  normal. Cutout keeps what its alpha test needs. Exact: the shadow map is
  unchanged.
- **Item 4: shared placeholders (Q13).** Every layer whose quad count falls in a
  bucket draws the bucket's one placeholder; the layer's header gains its quad
  count, which the shader reads and degenerates the padding quads. Buckets grow
  by 2^(1/4), about 9% padding on average, chosen from the fixtures' quad counts
  before any GPU run; two-byte indices up to 16,384 quads, four past them. On
  Metal this turns one draw per Chunk layer into one per bucket in every pass
  and cascade, frees the placeholder's 16 bytes a quad and its upload. Opaque
  and cutout gain the draws; blended layers are sorted and drawn one by one, so
  they gain the memory only. ADR-0034's "each layer has its own placeholder"
  changes with it.
- **The image (Q3).** 0, 1d and item 4 are exact: their captures differ from
  the commit before only where, and by no more than, two runs of that commit
  differ from each other (decided 2026-11-20: on Metal's fast math a
  shadow-only shader cannot be bit-identical, and 1d's first build moved 1 to
  31 shadow-edge pixels by up to 7 levels against 100 to 1,600 between base
  runs). 1a, 1b and 1c change distant shadows: each is shown to you as before
  and after captures of `flight`, `stationary` and `hidden-hill`, with crops
  of distant canopy for 1a, and lands only if you accept it.
- **No coarse casters (Q2).** The far cascades lie inside level 0's ring, where
  no coarse mesh exists, and coarse ground would shadow the finer ground it
  covers.
- **1c, how a cascade is kept (Q14).** Two public seams: a system after Bevy's
  writes World-anchored `Cascades`, and a cascade that need not redraw has its
  render phase taken out before Bevy's early shadow pass and put back after, so
  its pass never begins and its layer keeps last frame's depth. This relies on
  Bevy's texture cache returning the same shadow texture; the texture's id is
  compared every frame, and a change forces a redraw. Both seams are listed in
  DESIGN.md's `bevy_pbr` friction points.
- **1c, the cached cascades' shape (Q15).** Centred on the camera, not the Eye:
  cascades are chosen by depth from the camera, which third person puts behind
  the Eye. Each covers the sphere of the cascade's far bound plus a 25% margin,
  seen from the sun, so turning never invalidates one; the margin is adjusted on
  captures.
- **1c, what they hold (Q4).** Terrain only. The near cascades are redrawn every
  frame with every caster, so avatars and drops cast within 25 m only.
- **1c, when one redraws (Q5, Q17)**: when the camera leaves its margin, when a
  level-0 Chunk inside it lands, is replaced or is released, or when the sun's
  direction changes at all; at most one cached cascade a frame, taking turns,
  except that a changed shadow texture redraws every one at once. A newly landed
  Chunk's distant shadow may lag a frame or two.

## Testing Decisions

- **Gate (Q7).** Packed quads ticket 06's method: interleaved timing rounds and
  a separate `--instruments gpu` series, release `profiling` builds, quiet host,
  coordinated with other threads that benchmark. Each series interleaves
  `origin/main`, the commit before the item and the item; the item is gated
  against the commit before it. Fixtures `flight`, `active`, `transparency`,
  `stationary` and `sprint-flight`. An item passes when its target falls beyond
  run spread in the GPU series and frame p50 and p95 are no worse in any fixture
  in both rounds.
- The active fixture's fire goes out at 22-26 s on `main`; every series runs
  under an untracked runner copy with only that assertion removed.
- No CI test compiles WGSL: launch the game after any shader change.
- Tests: 1a and 1c's list edits and redraw rules as Bevy app tests (which
  entities each cascade holds; which cascades redraw for a landing, a move, a
  turn, a sun change, a texture change); item 4's bucket choice and the
  padding's degeneration in the expansion test.
- Captures (`TESSERAE_SCREENSHOT_OFFSCREEN=1`) in one `/tmp` folder, never
  committed.

## Out of Scope

- A day cycle, or moving the sun.
- Changing which levels exist where, or the coverage rule (ADR-0019).
- Multi-draw indirect (packed quads ticket 05: a CPU loop on Metal).
- Step 22, LOD features (Q8).
- Any Mod-visible change.

## Further Notes

- **Step 22 (LOD features) stays out (Q8)** and follows on its own branch and
  spec: it adds work on purpose, so it cannot pass this branch's gate. Where the
  two touch:
  - 1b chooses the cascade distance against today's view distance; step 22
    re-checks it when it raises the default.
  - Coarse levels cast nothing (Q2); whether step 22's impostors cast is its
    decision, and 1c's caching must not rule it out.
  - This branch's savings are the GPU headroom step 22 spends; its final numbers
    are step 22's starting budget.
- The Bevy 0.20.1 facts above were read from `bevy_pbr`, `bevy_render`,
  `bevy_core_pipeline`, `bevy_light` and `wgpu-hal` 30.0.1 source on 2026-11-20.
  Unsettled: whether naga's Metal output adds the base vertex to `vertex_index`;
  item 4 relies on `first_vertex_index` either way.
