# 01: Markers and baseline

Status: done
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
- [x] The baseline numbers above, with provenance, in Comments.
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

Baseline (2026-11-21, 20:06 to 20:47): `origin/main` `5285969a` against the
branch head `8be55f68` (01 to 05, 04 reverted, leaves to 61 m), release
`profiling` builds (sha256 `75df9357` and `87f705b6`), offscreen 1920x1080, AC
power, no thermal warning, load 5 to 6.7. Timing series main, head, head, main;
then a GPU series and a `no-shadows` GPU series of each. Runs cut on 2026-11-21
to save time: logging-on runs only (the off runs feed only `overhead.json`) and
three fixtures; `active` tracked `transparency` in every earlier series.

| ms | main | head |
| --- | --- | --- |
| frame p50, flight | 3.60 | 3.24 |
| frame p50, transparency | 4.09 | 3.27 |
| frame p50, stationary | 3.36 | 2.82 |
| frame p95, flight | 6.09 | 6.01 |
| frame p95, transparency | 5.27 | 5.28 |
| frame p95, stationary | 5.01 | 4.73 |
| main CPU p50, flight / transparency / stationary | 0.82 / 0.77 / 0.75 | 0.84 / 0.84 / 0.74 |
| render CPU p50, flight / transparency / stationary | 1.41 / 1.38 / 1.32 | 1.31 / 1.31 / 1.19 |
| GPU frame p50, flight | 4.11 | 3.70 |
| GPU frame p50, transparency | 4.63 | 3.59 |
| GPU frame p50, stationary | 4.07 | 2.98 |
| GPU frame p50 under `no-shadows`, flight / transparency / stationary | 2.66 / 2.68 / 2.21 | 2.53 / 2.59 / 2.13 |
| shadows' cost, flight / transparency / stationary | 1.45 / 1.95 / 1.86 | 1.17 / 1.00 / 0.85 |

Draws a frame, Player camera and cascades 0 to 3 (main's from 01's build, which
draws as main does): `flight` 267 and 4, 14, 57, 143 against 127 and 4, 11, 29,
26; `stationary` 241 and 3, 7, 48, 131 against 123 and 2, 4, 32, 31. Held
Chunk mesh bytes: 24.1 against 11.0 MB. The shadows still cost 0.85 to 1.17 ms
at the head: what 06 and 07 go after.
