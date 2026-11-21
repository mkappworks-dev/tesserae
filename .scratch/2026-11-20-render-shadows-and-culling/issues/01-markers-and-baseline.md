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

- [ ] ~~The shadow draws have their own GPU span~~, and frame-profiling.md says
      what each span holds. Dropped 2026-11-21, see Comments.
- [x] Draws per frame counted for the camera and each cascade.
- [ ] The baseline numbers above, with provenance, in Comments.
- [ ] `cargo fmt --all --check`, `npm run fmt:check`, `scripts/cargo-ci` green.

## Comments

Shadow markers dropped (2026-11-21). Built as `shadow_start` after Bevy's
`early_prepass_build_indirect_parameters` and `shadow_end` after
`per_view_shadow_pass::<EARLY_SHADOW_PASS>`, they misled and cost:

- The span missed the work it named. 03 took about 0.7 ms off the GPU frame
  while the shadow span moved 0.164 to 0.155 ms; the saving showed in
  `prepass_end` to `main_start`, because the GPU overlaps the cascades' passes
  with what follows. The span read about a tenth of what `no-shadows` takes off.
- They cost 0.1 to 0.25 ms of GPU frame with `--instruments gpu`: at the branch
  head, `stationary` and `transparency` read 2.92 to 3.14 and 3.64 to 3.73 ms
  with them against 2.78 to 2.96 and 3.41 to 3.52 with main's markers.
- The hand camera's view reading 2 ms against 0.72 on main is not theirs: it
  shows with main's markers too. Shared GPU work that main left outside both
  views lands inside the hand camera's; the frame total is unchanged.

So the markers are main's again, with the comment corrected, and the shadows'
cost is the GPU frame's difference under `no-shadows`. The draw counts stay:
they showed 05 halving the draws.
