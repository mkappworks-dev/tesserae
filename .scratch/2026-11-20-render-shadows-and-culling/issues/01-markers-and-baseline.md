# 01: Markers and baseline

Status: ready-for-agent
Blocked by: a quiet host (coordinate with other benchmarking threads)

Spec: [Shadows and culling](../spec.md).

## What to build

Name the shadow pass in the GPU markers, then take the numbers every later gate
reads.

- A GPU marker ordered right after Bevy's early shadow pass, so the four cascade
  draws get their own span; the existing markers keep their names.
  `docs/performance/frame-profiling.md` lists the new column and says what each
  span holds.
- A `profiling` count of indirect draws per frame, for the camera and for each
  cascade (bins in each view's binned phases). `visible_meshes_camera*` counts
  entities, which ticket 05 leaves unchanged while it collapses draws.
- Correct the comment in `profiling/gpu.rs` that places the shadow passes
  between `prepass_end` and `main_start`: the drawing pass is before
  `prepass_end`, and what follows it is compute.
- Baseline on `origin/main` (`5285969a`), release `profiling`, ticket 06's
  method: an interleaved timing series, a separate `--instruments gpu` series,
  and one series with `--switches no-shadows` for the shadows' whole cost.
  Fixtures `flight`, `active`, `transparency`, `stationary`, `sprint-flight`.
- Record in Comments: frame p50 and p95, main and render CPU, GPU frame and
  every span, the no-shadows delta, and draws per view. Note load, power and how
  each was taken.
- Gate method for tickets 02 to 07: interleave three binaries, `origin/main`,
  the commit before the item and the item's commit. The item is gated against
  the commit before it; `origin/main` anchors the series.

## Acceptance criteria

- [ ] The shadow draws have their own GPU span, and frame-profiling.md says what
      each span holds.
- [ ] Draws per frame counted for the camera and each cascade.
- [ ] The baseline numbers above, with provenance, in Comments.
- [ ] `cargo fmt --all --check`, `npm run fmt:check`, `scripts/cargo-ci` green.
