# 04: Cut partly covered coarse faces at upload

Status: ready-for-agent
Blocked by: 03

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-upload-cut`.
Conditional: run only if, after 03, GPU time in coarse Chunks drawn with the
coverage test still passes the effect gate when the test is switched off;
otherwise `wontfix` with the numbers.

## What to build

Apply ADR-0019's rule to geometry instead of fragments for the coarse Chunks
03 left on the discarding variant, so that no opaque or cutout pipeline needs
the coverage test.

- At upload, the Client drops a partly covered coarse Chunk's quads that lie
  wholly under drawn finer Chunks, and splits a quad that crosses a finer
  Chunk's boundary along it, keeping the uncovered part. The test is the same
  as the shader's: the Voxel behind the face.
- When the drawn mask changes under a coarse Chunk, its mesh is cut again
  from the whole mesh the Server sent, which the Client keeps. The cut and the
  mask change land in the same frame, so no frame draws the old cut against
  the new mask.
- The work runs where uploads run today, off the main thread (streaming
  ticket 14); its cost is reported per frame.
- The Server still sends whole meshes (ADR-0002). Amend ADR-0019: the rule
  is unchanged, applied to geometry for opaque and cutout, to fragments for
  blended.

## Acceptance criteria

- [ ] Unit tests: a quad wholly covered, wholly uncovered, and crossing a
      finer Chunk's boundary; a re-cut when a finer Chunk lands and when it is
      released.
- [ ] Offscreen captures pixel-identical to 03's, as in 03.
- [ ] Zero rule-hole frames in every fixture, zero fill-hole frames in `dive`
      and both sprints.
- [ ] Offscreen 1440p passes the effect gate against 03; the cut's CPU cost
      per frame recorded, and main-thread p99 does not regress.
- [ ] ADR-0019 amended.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
