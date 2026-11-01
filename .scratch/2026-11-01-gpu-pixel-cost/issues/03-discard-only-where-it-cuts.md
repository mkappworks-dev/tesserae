# 03: Discard only where the coverage rule cuts

Status: ready-for-agent
Blocked by: 02

Spec: [GPU cost per pixel](../spec.md).
PR boundary: own branch `tech/gpu-discard-variants`.
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

- [ ] Unit tests: level 0, an uncovered, a partly covered and a wholly
      covered coarse Chunk each take the right variant or are hidden, and
      each follows a finer Chunk landing and unloading.
- [ ] Offscreen captures pixel-identical to the before binary: `stationary`,
      the far view (lift 90 m, pitch -5) and `teleport` 1.2 s after the jump.
- [ ] Zero rule-hole frames in every fixture.
- [ ] Offscreen 1440p passes the spec's effect gate against 02's baseline;
      offscreen 1080p does not regress.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
