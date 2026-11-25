# 04: Screen-space ambient occlusion

Status: done

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 4: Bevy's screen-space ambient occlusion on the game camera,
measured before it lands. The chunk material already has a prepass with a
normal output (`chunk_prepass.wesl`), which screen-space AO reads. It missed
the frame budget on every tier, so the user chose (Q60) a player setting, off
by default, with the prepass cut to the levels AO can reach.

## Acceptance

- [x] GPU frame time on `flight` against `main` recorded.
- [x] Canopies and building corners darken where blocks meet, and the jungle
      floor gets no darker than 02 leaves it; approved on captures.
- [x] A player setting, off by default; its cost with the cut prepass recorded.

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

2026-11-25: landed as the Q60 setting. "Ambient occlusion" under Display, Image
quality, off by default; when on, SSAO Low on Low, Medium and Custom and SSAO
Medium on High; never under MSAA. The camera's depth and normal prepass now
draws level 0 only (coarse quads get no area in its vertex stage), so the
coverage cut it needed for coarse faces is gone; shadow passes still draw every
level.

Same method against 79ec13cf, the setting on in both runs' profile (High,
SMAA). Runs were bimodal: whole series in which the second camera, which AO
does not touch, slowed 2.5x (the GPU clock state; base hit it once, the branch
in 6 of 9 Medium runs). Medians of the fast-state runs, ms:

| Tier   | Frame p50   | Frame p95   | Main camera | Prepass     |
| ------ | ----------- | ----------- | ----------- | ----------- |
| Medium | 4.02 → 4.56 | 4.74 → 7.08 | 0.78 → 1.76 | 0.20 → 0.68 |
| High   | 4.11 → 4.63 | 5.19 → 5.71 | 0.81 → 2.05 | 0.18 → 0.66 |

The cut saves little: the main camera's added cost went from 1.10 to 0.98 ms on
Medium and from 1.43 to 1.24 ms on High, since level 0 is almost all of the
prepass (its share is about 0.5 ms either way). The SSAO passes are the rest.
Captures: `/tmp/colour-04/v2/`, before 79ec13cf and after with the setting on,
each fully loaded (level 1 at 6743 Chunks on both sides).

Two fixes the cut prepass needed, found on those captures: with a normal
prepass bevy_pbr's main pass takes N from it, which is empty for a coarse face
and the floor's for water, so the chunk material drops `LOAD_PREPASS_NORMALS`
and keeps its own face normal; and a coarse face ignores the AO texel, which
the prepass never wrote. The table above predates both; neither changes the
cost. Jungle floor mean 27.98 → 27.67.
