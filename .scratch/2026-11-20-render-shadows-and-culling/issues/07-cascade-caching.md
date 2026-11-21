# 07: Cascade caching (1c)

Status: wontfix
Blocked by: 06

Spec: [Shadows and culling](../spec.md).

## What to build

Keep the far cascades between frames and redraw one only when it must.

- **Shape.** A far cascade is centred on the camera, not the Eye: cascades are
  chosen by depth from the camera, which third person puts behind the Eye. It
  covers the sphere of its far bound plus a 25% margin, seen from the sun,
  snapped to texels; turning never moves it. Casters nearer the sun than the box
  are clamped onto its near plane by the shadow pipeline's unclipped depth, so
  the box need not reach them. A system after Bevy's cascade build writes these
  `Cascades` for the far cascades; near ones stay Bevy's.
- **Contents.** Terrain only: a far cascade's visible list keeps Chunk layers
  and drops avatars, drops and anything else that moves.
- **Redraw** when the camera leaves the margin, a level-0 Chunk inside it lands,
  is replaced or is released, or the sun's direction changes; at most one far
  cascade a frame, taking turns.
- **Skip.** A cascade that needs no redraw has its shadow render phase taken out
  before Bevy's early shadow pass and put back after, so its pass never begins
  and its layer keeps last frame's depth. Never drop the phase: Bevy queues only
  new or changed entities into it.
- **Guard.** The shadow texture's id is compared every frame; a change redraws
  every far cascade that frame, past the one-a-frame limit, since a new texture
  holds nothing.
- A skipped cascade still runs its GPU culling and indirect parameter builds;
  report what they cost, and skip them too in a follow-up only if they matter.
- DESIGN.md's `bevy_pbr` friction points list both seams, so a Bevy upgrade
  checks them.

## Acceptance criteria

- [ ] Bevy app tests: which far cascades redraw for a landing inside and
      outside, a move within and past the margin, a turn, a sun change and a
      texture change; one a frame at most, except after a texture change; a far
      cascade holds no avatar.
- [ ] Gated by 01's method, `stationary` and `sprint-flight` both reported;
      numbers in Comments.
- [ ] Before and after captures of `flight`, `stationary` and `hidden-hill`, and
      a capture right after a Chunk lands in a far cascade, shown to you; lands
      only on your approval.

## Comments

Not built (2026-11-21): its ceiling is too small. After 03 (leaves to 61 m) and
06 (three cascades, 10, 61 and 150 m) the branch head was run against the same
build with its far cascades holding nothing, which is more than caching could
save: caching keeps the GPU culling and indirect builds and redraws on landings
and moves. GPU series in mirrored order, three fixtures, mean of six runs, ms:

| | head | cascade 2 empty | cascades 1 and 2 empty |
| --- | --- | --- | --- |
| GPU frame, flight / stationary / sprint-flight | 3.37 / 2.79 / 3.26 | 3.41 / 2.83 / 3.28 | 3.22 / 2.76 / 3.18 |
| frame p50 | 3.28 / 2.86 / 3.31 | 3.26 / 2.86 / 3.27 | 3.07 / 2.79 / 3.14 |
| frame p95 | 5.86 / 4.57 / 6.80 | 5.68 / 4.51 / 6.80 | 5.45 / 4.50 / 6.68 |

Cascade 2 (61 to 150 m, 31 opaque draws in `stationary`) costs nothing
measurable. Emptying cascade 1 too saves at most 0.03 to 0.15 ms of GPU frame,
and caching it would redraw often (10 to 61 m moves with the camera) and stop
avatars and drops casting past 10 m, for two seams in Bevy's internals. Reopen
when step 22 adds casters at distance or the view distance grows enough that
the far cascade's draws show in the shadows' cost (GPU frame with shadows
against `no-shadows`).
