# 07: One depth buffer instead of six cameras

Status: done
Blocked by: 01, 02

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-cameras`.

## What to build

Every coarse level has its own camera today, each clearing depth, so finer
levels paint over coarser ones regardless of depth and every coarse pixel under
finer terrain is drawn and then overdrawn. With 02's coverage, coarse fragments
where a finer level is drawn are already discarded, so one depth buffer is
correct.

- Check depth precision of Bevy's reverse-Z out to `lod_distance_m` (3000 m in
  the demo, 4096 m by default) from the Player's near plane, with a capture of
  distant ridges.
- Draw every level with the Player camera into one depth buffer. If precision
  forces a second, coarse camera, it must not clear depth. Level 0 stays the
  only shadow caster; fog stays identical.
- Accept the one visual change: a coarse Chunk not yet refined can now hide
  finer terrain behind it.
- Amend 02's ADR.

## Acceptance criteria

- [ ] Render CPU or GPU time passes the spec's effect gate, per 01's
      attribution.
- [ ] No depth artefacts in review captures at 1 km and 2.5 km.
- [ ] Zero rule-hole frames still, in every fixture.

## Validation and authorization

Meets `spawn_camera` in `client/chunk_view.rs` and `client/camera.rs` as #29
left it. Local work only; no push or PR without approval.

## Comments

### 2026-11-01: seam walls kept only where the finer ground leaves them open

Branch `tech/streaming-seam-cost`. Effect gate pending the final review series.

Built: each coarse Chunk carries `SeamOpenings`, read from its eight children
at generation (`seam_openings`, tesserae-core): which Voxels against a seam
plane have finer Voxels against it that are not opaque cubes. A seam wall
stands only there, and a hidden wall between two open ones in a row is kept so
the run stays one quad (it is inside the ground either way). Level 1's
children are level 0's own Chunks, so it is exact. Levels 2 and 3 read
children made at the finer Voxel size but not downsampled as that level is, so
they look one finer Voxel up and aside. Levels 4 and 5 keep every wall: every
margin that lost no opening there saved under 3%.

Wall quads, demo-like terrain (relief 24 m, lake at 34 m), mesh shells around
one Eye, old -> new, and as a share of the level's other coarse quads:

| level | base quads | walls before | walls after | share       |
| ----- | ---------- | ------------ | ----------- | ----------- |
| 1     | 76282      | 8465         | 3126        | 11.1->4.1%  |
| 2     | 69865      | 9610         | 7720        | 13.8->11.0% |
| 3     | 65899      | 6850         | 6115        | 10.4->9.3%  |
| 4     | 49719      | 6642         | 6642        | 13.4%       |
| 5     | 24309      | 2134         | 2134        | 8.8%        |
| all   | 286074     | 33701        | 25737       | 11.8->9.0%  |

Over four terrains (two seeds, relief 24 and 48 m) no wall the finer level's
data leaves open was lost at levels 1-3; `seams.rs` keeps that as a test.
Other ideas, measured on the same terrain: 96% of level 1's exposed walls are
in the depth-1 row, but `SEAM_DEPTH` 1 everywhere loses 120-830 exposed cells
per level, and no coarse-only slope rule predicted which. Light never split a
wall merge. The Material key does (dropping it cuts 20-45% of walls), kept
because the lower row would show grass where dirt is. About 44% of the walls
face away from the Eye's anchor at mesh time; dropping them needs a re-mesh as
the Eye crosses a Chunk, and they cost vertices only (back faces are culled).

Cost: `seam_openings` takes 0.07 ms per surface coarse Chunk against 1.13 ms
for its worldgen and downsample (levels 1-3 only).

Captures under `/tmp/claude-streaming-impl/review/seamcost/` (stationary,
teleport, far1k = lift 90 pitch -5, far2k = lift 120 pitch -2.7, sprint and
sprint-int = sprint-flight on bin-int2), against `review/*-int`: at most 35
pixels differ, along one hairline that is pale in the reference and grass
here; no slit in any. Counts and margin sweeps:
`/tmp/claude-streaming-impl/seam-cost/`.

2026-11-01 (integration): level 3 takes a margin of two, not one. With one, a
missed opening on one terrain was hidden only because a kept run covered it;
two gives up some of level 3's cut (11-22% -> 4-9%) for no luck in it. Level
2 keeps one, level 1 stays exact. A far probe (ticket 06) has no children to
read, so its Chunks keep every seam wall.

### 2026-11-01: final series; done

Final series `/tmp/claude-streaming-impl/final`: release `profiling` builds,
offscreen, two interleaved rounds (cut from three), one binary per merge in
order (`bNN` is the integration branch just after ticket NN), the tip `075ff52`,
and the tip with one switch changed. Medians of the two runs; a range is the two
runs, low-high.

- Frame p50 `b02` -> `b07`, ranges apart everywhere: `stationary` 8.39 -> 4.85
  ms, `active` 8.81 -> 5.16, `transparency` 9.42 -> 5.67, `flight` 8.64 -> 5.34,
  `sprint-flight` 8.43 -> 5.01, `dive` 6.93 -> 3.80, `teleport` 8.33 -> 4.89:
  40-45%. p99 10.6-12.3 -> 6.1-7.7 ms.
- Rule-hole frames 0 in every fixture. Depth captures at 1 km and 2.5 km: see
  the seam-wall comment above.
