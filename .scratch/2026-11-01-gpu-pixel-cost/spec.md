# GPU cost per pixel: 144 Hz presented at 1440p

Status: ready-for-agent. Drafted 2026-11-01 for review; no ticket is
authorized until you approve the spec and the breakdown.

Source: branch `tech/gpu-pixel-cost`, cut from `tech/streaming-integration` at
`fff9788`, whose PR to `main` is not merged yet. Builds on the streaming
performance spec, `.scratch/2026-10-29-streaming-performance/`, tickets 01,
02, 07, 10 and 15. Tickets: [issues/](issues/).

## Problem Statement

On the 144 Hz external display the engine does not reach 144 Hz. In 01's
presented run (2560 x 1440 surface, `Immediate` present, the tip `df93f3b`),
frame p50 is about 9 ms and p95 15.5 to 17.4 ms in `stationary`, and similar in
both sprints. Offscreen at 1920 x 1080 the same build has p50 4.6 ms and p95
6.2 ms, inside the 6.9 ms a 144 Hz frame allows (ticket 15).

The CPU is not the cost. In presented frames over 16.7 ms the main thread
spends about 0.6 ms, the render thread about 1.4 ms and mesh upload under
0.1 ms. The rest is the GPU and presentation. 1440p has 1.78 times the pixels
of 1080p and the presented p50 is about 1.96 times the offscreen one, so the
frame scales with pixels, plus something.

What nobody has measured yet is where in the GPU frame that time goes at
1440p, and how much of the p95 tail is GPU work and how much is waiting to
present.

## Solution

Measure first, then cut the cost per pixel, cheapest exact cut first:

1. Check that Bevy 0.20.1, already on `main`, does not move the tip's image
   or timing, so every measurement in this spec runs on one Bevy.
2. Attribute GPU time per pass at 1440p, presented and offscreen, and test the
   candidate causes one at a time with diagnostic switches.
3. If the coverage discard is a cost: draw without it wherever it cuts
   nothing, and stop drawing coarse Chunks it would cut entirely. The image
   does not change.
4. If the boundary ring still costs: cut coarse faces under finer levels on
   the Client at upload, so no pipeline needs the discard.
5. If the multisampled targets are a cost: choose the anti-aliasing on
   captures.
6. A render scale with upscaling, as a setting whose default you choose on
   captures.

Then check the target presented and offscreen.

## User Stories

1. As a player on a 144 Hz 1440p display, I want a frame every 6.9 ms, so that
   motion is as smooth as the display allows.
2. As a player, I want the frame rate to hold while sprinting and diving, so
   that fast movement does not stutter.
3. As a player, I want no holes in the ground while levels stream in, so that
   the speed-up does not cost what the streaming spec fixed.
4. As a player on a smaller or 1080p display, I want no slower frame than
   today, so that this work is not paid for by other setups.
5. As a player, I want the image to stay as it is unless I accept a change,
   so that quality is never traded silently for speed.
6. As a player, I want a render scale setting, so that I can trade sharpness
   for frame rate on a weaker GPU or a larger display.
7. As a player, I want the menus, HUD and text drawn sharp at the display's
   own resolution whatever the render scale, so that only the World softens.
8. As a player, I want distant terrain to keep its silhouette at a reduced
   render scale, so that the far levels stay readable.
9. As the developer, I want GPU time per pass at 1440p, so that I optimize the
   pass that costs and not the one I suspect.
10. As the developer, I want to know whether the presented p95 tail is GPU
    time or presentation, so that I do not cut pixels to fix a pacing problem.
11. As the developer, I want an offscreen 2560 x 1440 run that works with the
    screen locked, so that I can iterate without you at the Mac.
12. As the developer, I want each candidate cause switchable alone in a
    profiling build, so that its cost is measured, not inferred.
13. As the developer, I want every pixel-identical change proven identical on
    captures, so that a speed-up cannot hide a rendering change.
14. As the developer, I want every quality-changing change shown to you on
    captures before it ships, so that you decide the trade.
15. As the developer, I want ADR-0019's coverage rule to keep holding whatever
    implements it, so that a fragment is still cut exactly where a finer level
    draws.
16. As the developer, I want the Bevy patch release in before the baseline,
    so that before and after differ only by our changes.
17. As the maintainer, I want the profiler's GPU markers kept out of timing
    series, so that their own 16 to 21% cost is not read as the engine's.
18. As the maintainer, I want presented runs requested only where offscreen
    cannot answer, so that your time at the Mac is spent once per checkpoint.
19. As a Mod author, I want nothing in the WIT contract or my Declarations to
    change, so that this work is invisible to Mods.

## Implementation Decisions

- **Bevy 0.20.1 is already on `main`** (#49, 2026-10-28 release: ten commits
  on 0.20.0, wgpu stays 30.0.1). It also cleared the `ruzstd` advisory ignore.
  Ticket 01 only checks that it leaves the tip's image and timing alone, and
  raises the `bevy` requirement in `crates/tesserae/Cargo.toml` from 0.20.0 to
  0.20.1. A 0.21 does not exist yet; moving to it is a separate spec.
- **Attribution names the work.** GPU markers (`--instruments gpu`) split the
  frame into prepass, shadows and setup, opaque, transparent and post, plus
  the GPU span of the whole frame. The markers cost 16 to 21% of median, so
  attribution series and timing series stay separate (frame-profiling.md).
- **Diagnostic switches, profiling builds only.** Each changes one thing,
  default off, read once at startup, and is compiled out of normal builds:
  - no coverage discard: the coverage test is not compiled in, so coarse
    ground stands over finer ground. The image is wrong; the time is the
    discard's cost.
  - level 0 only: coarse levels are not drawn, a lower bound for their cost.
  - MSAA off: the Player camera gets `Msaa::Off`.
  - no shadows: the sun casts no shadow map.
  These are measurement aids, never shipping options.
- **Offscreen at 1440p.** The runner gains a size option for its offscreen
  target (the engine already reads the capture size from the environment), so
  2560 x 1440 runs with the screen locked. It is a proxy: presented runs stay
  the gate, since they add compositing and presentation.
- **What the code shows today**, the leads attribution tests:
  - The terrain fragment shader holds a `discard` for the coverage rule on
    every Chunk, level 0 included, though at level 0 it never fires. On
    Apple's tile-based GPUs a pipeline that may discard keeps hidden-surface
    removal from culling fragments before they are shaded. Unproven here.
  - The Player camera sets no `Msaa`, and Bevy 0.20 requires one on every
    camera with `Sample4` as the default: 4x MSAA is on, never chosen. At
    1440p that is 14.7 million colour and depth samples, stored and reloaded
    at every render pass boundary. The GPU markers split passes too, which may
    be why they cost 16 to 21%.
  - The sun has Bevy's default shadows: four cascades of 2048 squared. Their
    cost follows level 0's geometry, not the window's pixels.
  - 1920 x 1080 is exactly 0.75 of 2560 x 1440 on each axis, and offscreen it
    already meets 6.2 ms at p95. A 0.75 render scale is therefore likely
    enough alone; the exact cuts come first so that a higher scale, or none,
    suffices.
- **Discard only where it can fire.** Two variants of each terrain material:
  one with the coverage test, one without. Level 0, and coarse Chunks no finer
  level draws any part of, take the variant without. A coarse Chunk whose
  every finer Chunk is drawn is not drawn at all, because each of its
  fragments would be discarded. Both follow the drawn mask as it changes, in
  the frame the mask changes. The blended pass keeps the test: it does not
  use hidden-surface removal and must not draw water twice.
- **Cut at upload if the boundary still costs.** For a coarse Chunk partly
  covered, the Client drops the quads that lie wholly under drawn finer Chunks
  and splits quads that cross a finer Chunk's boundary, at upload, and redoes
  it when the mask changes under that Chunk. It is Client presentation, as
  retention is (ADR-0002); the Server still sends whole meshes. ADR-0019 is
  amended: the rule is the same, applied to geometry instead of fragments.
- **Render scale.** A Client setting: the World is drawn at a fraction of the
  window on each axis and upscaled to the window; UI is drawn after, at full
  resolution. Choices 1.0, 0.85, 0.75 and 0.67. Bevy 0.20's
  `MainPassResolutionOverride` (used by its DLSS path) or a scaled
  intermediate target are the two ways in; the ticket picks one by what keeps
  fog, tonemapping and the capture path working. The upscale filter is
  bilinear, optionally followed by Bevy's contrast-adaptive sharpening. The
  default stays 1.0 until you choose otherwise on captures.
- **Anti-aliasing is your choice.** If MSAA is a dominant cost, the candidates
  are MSAA 2x, FXAA and SMAA (both in Bevy 0.20), at native scale and at the
  render scale chosen. Blocky edges alias more than most scenes, so captures
  decide, not timing alone.
- **Fullscreen and presentation** are not changed by this spec. If ticket 02
  finds the p95 tail is presentation rather than GPU time, it proposes a
  ticket for that instead of cutting pixels.

## Targets

At unchanged view distance, LOD reach and visual quality except where you sign
off a change, in each fixture's measured phase (22 to 32 s), release
`profiling` builds on this Mac:

- **Presented, 2560 x 1440 surface at 144 Hz:** frame p95 at most 6.9 ms in
  `stationary`, `flight`, `sprint-flight` and `sprint-walk`, at the render
  scale and anti-aliasing you sign off. Native-scale numbers are reported
  beside them.
- **Offscreen, 1920 x 1080, every fixture:** no regression against the tip:
  median not worse beyond the run ranges, no consistent p99 regression.
- **Holes:** zero rule-hole frames in every fixture; zero fill-hole frames in
  `dive` and both sprints, presented and offscreen.

Each optimization ticket passes the streaming spec's effect gate on the
offscreen 1440p proxy: median at least 5% better, beyond the logging overhead
bound and between-run variation, with no regression in p99, streaming
readiness or simulation progress. Ticket 07 checks the targets.

## Testing Decisions

- **The highest seam is the rendered fixture**, `scripts/profile-frames`: it
  measures frame time, GPU passes and holes from outside the engine. Every
  ticket's acceptance is a series there, not a micro-benchmark.
- **Pixel identity is the second seam.** 03 and 04 must not change the image:
  offscreen captures before and after, same binary flags, compared pixel by
  pixel, in `stationary`, a far view (lift 90 m, pitch -5) and `teleport`
  mid-refill. Nonzero differences are explained or the ticket fails.
- **Unit tests at the drawn-mask seam**, as `drawn_mask` and `chunk_view`
  tests do today: which material variant a Chunk takes and whether it is drawn,
  for level 0, an uncovered coarse Chunk, a partly covered one and a wholly
  covered one, and that each follows the mask when a finer Chunk lands or
  unloads. For 04, quad cutting against a mask, including quads that cross a
  finer Chunk's boundary. Test outcomes, not how the variant is picked.
- **The hole counter** keeps guarding ADR-0019: `--instruments holes` in every
  timing series.
- **Quality changes** (05, 06) are judged by you on captures from one `/tmp`
  folder, at native scale and each candidate, never committed.

## Out of Scope

- Bevy 0.21 or any major engine upgrade.
- Changing view distance, LOD reach, Voxel size or what the far levels hold.
- Temporal upscaling or anti-aliasing (TAA, DLSS, MetalFX): Bevy's DLSS is
  NVIDIA-only and MetalFX has no binding here.
- Present mode, fullscreen and frame pacing, unless 02 makes them a ticket.
- Shadow quality, unless 02 names shadows dominant; then 02 proposes a ticket.
- CPU-side work: main thread, Server, meshing and streaming order.
- Fluid, which has its own spec.
- Bevy's occlusion culling, removed in streaming ticket 10.

## Further Notes

- **Presented runs need you at the unlocked Mac.** Each ticket that needs one
  gives a copy-paste command with the output folder right after the script
  name, because `--fixtures` takes every argument after it. Expected: 02's
  attribution, 06's captures and 07's check.
- **Heavy jobs** (cargo build, test, clippy, bench, engine runs) go through
  `/tmp/claude-streaming-impl/heavy.sh`, which serialises them with another
  session on this Mac. Nothing is timed while a build runs.
- **Artifacts** stay under `/tmp/claude-gpu-pixel-cost/`, never committed.
- **The demo Game distribution** is the one 01's presented run used, under
  the `tech+streaming-performance-impl` worktree, pinned by the runner's
  hashes.
- **Branches.** This branch carries the spec, the tickets and ticket 01's
  result. Later
  tickets get their own `tech/gpu-<slug>` branches unless you say otherwise.

## Tickets

| #  | Ticket                                      | Blocked by     |
| -- | ------------------------------------------- | -------------- |
| 01 | Check Bevy 0.20.1 against the tip           | none           |
| 02 | Attribute GPU time at 1440p                 | 01             |
| 03 | Discard only where the coverage rule cuts   | 02             |
| 04 | Cut partly covered coarse faces at upload   | 03             |
| 05 | Choose the anti-aliasing                    | 02             |
| 06 | Render scale with upscaling                 | 02             |
| 07 | Check the targets                           | 03, 04, 05, 06 |
| 08 | Explain the two presented timing states     | 07             |

02 ends with an attribution checkpoint. 03, 04 and 05 are conditional on it
and may become `wontfix`; anything else it finds dominant becomes a new ticket
for your approval.

## Follow-up approved (2026-11-02)

07 completed its measurement check but missed the presented target. You
approved [08](issues/08-explain-presented-waits.md): attribute the two
presented timing states, then settle the 1080p tail check on a quiet host.
SMAA and native render scale remain the approved defaults.
