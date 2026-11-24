# 04: Screen-space ambient occlusion

Status: ready-for-agent

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 4: Bevy's screen-space ambient occlusion on the game camera,
measured before it lands. The chunk material already has a prepass with a
normal output (`chunk_prepass.wesl`), which screen-space AO reads. Kept only
if it fits the frame budget; otherwise report the cost and stop, and the user
decides on per-corner AO.

## Acceptance

- [ ] GPU frame time on `flight` against `main` recorded.
- [ ] Canopies and building corners darken where blocks meet, and the jungle
      floor gets no darker than 02 leaves it; approved on captures.
- [ ] Off on any Quality tier where it misses the budget.

## Comments

2026-11-24: measured; misses the budget on every tier it runs on, so not
landed. Branch `t3/screen-space-ambient-occlusion`, unmerged.

Built: Bevy's SSAO on the Player camera, set per Distance preset (Low off,
Medium SSAO Low, High SSAO Medium, Custom as Medium; off under MSAA), radius
1.5 m. The chunk prepass cuts covered coarse faces as the colour pass does, or
its depth hides the finer surface. Three look fixes, each needed to see it:
bevy_pbr bounces the occlusion off the white base colour before the texture is
applied (0.5 became about 0.9), so the chunk shader bounces it off the real
albedo; the sun takes half of it, or sunlit canopies stay flat; it fades with
sky light, or the jungle floor darkens 13% (now 1%).

`flight`, offscreen 1920x1080, `--instruments gpu`, release `profiling`, base
cbde224a against this branch, three interleaved rounds of three runs per cell,
host load 9 to 40 from other worktrees. Medians of the nine runs, ms:

| Tier   | Frame p50   | Frame p95   | Main camera p50 |
| ------ | ----------- | ----------- | --------------- |
| Medium | 3.98 → 4.31 | 4.55 → 5.58 | 0.80 → 1.89     |
| High   | 4.20 → 5.15 | 5.73 → 6.58 | 0.86 → 2.37     |

The budget is the gpu-pixel-cost spec's presented 2560x1440 at 144 Hz, `flight`
p95 at most 6.9 ms, last verified at 6.454 ms: 0.45 ms of headroom. SSAO adds
about 1 ms of p95 at 1080p, about 1.8 ms at 1440p's pixel count. The prepass
(depth and normals over every level) is about half of it. Captures, sheets and
runs: `/tmp/colour-04/`. The user decides on per-corner AO.
