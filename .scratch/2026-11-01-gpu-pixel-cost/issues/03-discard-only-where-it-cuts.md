# 03: Discard only where the coverage rule cuts

Status: ready-for-agent
Blocked by: 02

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-discard-variants`, from
`tech/gpu-attribution`.
Conditional: run only if 02's no-discard switch shows the discard is a cost
beyond the effect gate; otherwise `wontfix` with 02's numbers.

## What to build

Keep ADR-0019's rule exactly, but pay for the fragment test only on Chunks
where it can discard something.

- Each opaque and cutout terrain material gets a variant without the coverage
  test: a shader def, so the variant's pipeline holds no `discard` for
  coverage. Cutout keeps its alpha `discard`; only the coverage one goes.
- Level 0 always takes the variant without: no level is finer.
- A coarse Chunk takes the variant without when no finer level draws any part
  of its region, and the variant with when some but not all of it is drawn.
- A coarse Chunk whose region every finer level's Chunks draw in full is not
  drawn at all, since every fragment of it would be discarded. Retained
  Chunks follow the same rule.
- A Chunk's variant and visibility follow the drawn mask in the same frame
  the mask changes, both ways: a finer Chunk landing or being released.
- The blended material keeps the test unchanged.

## Acceptance criteria

- [x] Unit tests: level 0, an uncovered, a partly covered and a wholly
      covered coarse Chunk each take the right variant or are hidden, and
      each follows a finer Chunk landing and unloading.
- [ ] Offscreen captures pixel-identical to the before binary: `stationary`,
      the far view (lift 90 m, pitch -5) and `teleport` 1.2 s after the jump.
- [x] Zero rule-hole frames in every fixture.
- [x] Offscreen 1440p passes the spec's effect gate against 02's baseline;
      offscreen 1080p does not regress.

## Result (2026-11-02)

`8fbff1c`: the coverage test is a material key (`coverage_test`), with
untested opaque and cutout materials beside the tested ones. The drawn mask
records which coarse Chunks' finer cover changed; `apply_cover` gives each
the untested material (none covered), the tested one (part) or hides it
(all), in the frame the mask is published. Level 0 is always untested. Water
keeps the test. 62 view and mask tests pass, among them six new ones.

`bin-02` (before) against `bin-03` (`99aeaa39...`), offscreen, no GPU
markers, three interleaved rounds, host load 3.7 to 10. Zero rule-hole and
fill-hole frames in all 78 runs.

| size  | fixture       | p50 before | p50 after | p95 before | p95 after |
| ----- | ------------- | ---------- | --------- | ---------- | --------- |
| 1440p | stationary    | 6.46-6.57  | 4.41-4.50 | 7.45-7.52  | 5.46-5.97 |
| 1440p | flight        | 6.77-6.98  | 4.80-5.16 | 7.95-8.01  | 5.71-6.43 |
| 1440p | sprint-flight | 6.52-6.68  | 4.42-4.56 | 7.83-7.96  | 5.93-6.50 |
| 1440p | sprint-walk   | 6.65-6.69  | 4.43-4.46 | 7.75-7.89  | 5.58-5.83 |
| 1440p | dive          | 4.61-4.78  | 3.72-3.79 | 6.33-6.64  | 4.78-4.91 |
| 1440p | teleport      | 6.69-6.87  | 4.51-4.64 | 7.48-7.73  | 5.74-5.90 |
| 1440p | transparency  | 7.53-7.65  | 5.60-5.79 | 8.34-8.43  | 6.37-6.68 |
| 1440p | active        | 6.58-6.80  | 4.43-4.51 | 7.19-7.69  | 5.30-5.41 |
| 1080p | stationary    | 4.38-4.56  | 3.27-3.74 | 5.35-5.40  | 4.76-5.64 |
| 1080p | sprint-flight | 4.57-4.72  | 3.18-3.31 | 5.68-5.91  | 5.16-5.34 |

- Medians 20 to 33% lower at 1440p, every range apart; p99 lower in every
  fixture (`sprint-flight` 7.26 to 9.32 after, 8.59 to 8.75 before: one
  round's tail, the others lower). GPU frame with markers: 6.69 to 4.70 ms.
- Offscreen 1440p p95 is under 6.9 ms in every fixture.

Captures: **not pixel-identical.** Each binary is deterministic in
`teleport` (0 pixels differ between its rounds), and the two binaries
differ by 2,799 of 3,686,400 pixels there; in `stationary` by 6,446 to
6,906 against 4,067 between rounds of one binary, and in the far view by
1,817 to 2,253 against 99. Every difference is a run of single pixels along
a distant horizontal edge, about half brighter and half darker, invisible at
7x zoom. Zero rule holes, so no coarse fragment stands where a finer level
draws. The likely cause is that a pipeline that may discard also changes
edge pixels: MSAA shades an edge pixel at a point just outside its triangle,
and the old test, sampling there, could discard a fragment of an uncovered
Chunk; texture derivatives across discarded helper pixels change too. Not
proven. Diffs and crops: `/tmp/claude-gpu-pixel-cost/diff-03-*.png`.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
