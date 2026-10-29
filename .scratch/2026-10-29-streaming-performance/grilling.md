# Streaming performance: grilling record

Interview of 2026-10-29. Every recommendation below was accepted; where a later
round revised an earlier answer, the revision is listed under it. The spec
reconciles these into the current plan.

## Round 1

- Q0. Recreate `.worktrees/tech+streaming-performance` from `main` at
  `0c913c3` (the first one was deleted by another session before any write).
- Q1. One spec, tickets shipped as staged PRs, each with its own before/after.
  Persistence and visibility culling are gated, not committed.
- Q2. The demo's fluid cost is a separate later spec. Here it is a recorded
  confound: fluids share the three async workers with streaming.
- Q3. A coarser level draws a region until the finer level has delivered it.
  The metre cut in `chunk_material.wesl` and the 48 m coarse near plane go.
  A brief blocky patch is accepted in place of a hole.
  Revised by round 2 Q1: retention lives on the Client, in both directions.
- Q4. Dispatch order is decided on the Server from its own Player state: Eye,
  look and velocity. The Client sends nothing new.
- Q5. Where nothing is drawn, the coarsest missing level is dispatched first,
  and one priority spans the coarse levels instead of a level-1-first scan.
- Q6. Generate and light still run for the whole load ring; work behind the
  camera loses priority but is never skipped. The uniform air and buried solid
  skips stay. Visibility culling is a later, render-side, gated ticket.
- Q7. Cheaper coarse generation first means identical output. A column-only far
  level is a quality probe you approve on captures.
  Revised by round 4 Q5: profile before building the shared pass.
- Q8. Persisting coarse levels is a cache behind a measured gate. ADR-0011 is
  amended only if the gate passes.
- Q9. Stay inside bevy_pbr and `MaterialExtension`; no custom GPU-driven
  renderer.
- Q10. New profile fixtures, per-stage and per-level columns and a hole count.
  Revised by round 4 Q2: the hole count models the draw rule in force.
- Q11. Targets. Revised by round 4 Q4.
- Q12. Measure with PR 7's demo dist through `--game`, pinned by hash, and
  re-baseline when PR 7 merges.

## Round 2

- Q1. Retention is Client-side presentation in both directions: an unloaded
  mesh stays until other delivered meshes cover its region, or until it is
  outside every level's mesh shell.
- Q2. A per-level covered bitset at level k-1 Chunk granularity: delivered
  (empty included) or all eight children covered. Updated incrementally, in
  the `drawn_mask` layout, read by opaque, cutout and blended. Delivered means
  the mesh entity exists.
- Q3. Score: metres from the Eye predicted 1 s ahead (capped at the view
  distance) to the Chunk's nearest point, halved inside the view cone padded
  10%, Chunks within 2 Cells always in the cone, an age credit. Constants are
  tuned in the priority ticket.
- Q4. Re-score on an anchor Cell change, a look change over 15 degrees or a
  speed change over 25%; budget 0.2 ms of tick.
- Q5. The priority ticket changes order only. Caps are retuned with threads.
- Q6. Holes first, coarsest level first, then refinements by score. Level 0 and
  coarse keep separate caps.
- Q7. Relight orders columns by score and stays top-down within a column.
- Q8. The one-pass 64-cubed generation applies to untouched branches only;
  saved-change branches keep today's path. Bit-equal across seeds and levels.
- Q9. Persistence gate: coarse generation over 30% of worker CPU until settled,
  or a restart settle over 3 s, in restart and teleport-revisit fixtures.
- Q10. The column-only far level is a `ready-for-human` probe.
- Q11. The first render ticket collapses the per-level cameras once coverage
  makes one depth buffer correct; level 0 stays the only shadow caster.
- Q12. The visibility graph is counted before anything is culled; culling is
  built only if it removes at least 20% of drawn Chunks.
- Q13. Hole metric. Revised by round 4 Q2.

## Round 3

- Q1. The packed vertex decodes to exactly today's positions, normals and UVs
  for every Shape and fluid level, and each mesh carries an explicit `Aabb`.
  16 B if exact, else 20 B.
- Q2. Region batching merges opaque and cutout only; blended stays per Chunk
  for `blend_order.rs`.
- Q3. This branch ships the spec, all tickets and ticket 01. Every later
  ticket is its own `tech/streaming-<slug>` branch and PR.
- Q4. The coarse generation ticket is blocked by PR 7's merge (`worldgen.rs`).
  Tickets 01 and 02 rebase across the small `profile.rs` and `chunk_view.rs`
  overlaps and note them.
- Q5. The no-gap ticket adds an ADR for the drawing rule; the camera ticket
  amends it.
- Q6. Worker tuning measures 2 to 5 async threads, the pool split and coarse
  jobs yielding to level 0. A new pool only if it wins a matched A/B.
- Q7. Ticket 01 ends with an attribution checkpoint. Later tickets may be
  reordered, not re-scoped, with your approval.

## Round 4: review of rounds 1 to 3

You asked for the questions and decisions to be reviewed. Three decisions were
wrong as written and three questions had been missed.

- Q1. Which sprint stalls is unknown, so ticket 01 measures flight sprint at
  64 m/s (ADR-0008's cap, not 60), physics sprint at 8 m/s and the dive. The
  main display is a 144 Hz 2560x1440 monitor, so "about 30 fps" is a 28 to 35
  ms frame, not vsync halving from 60.
- Q2. The hole metric models the draw rule in force: today's metre cut
  discards coarse meshes that were delivered, so a delivery-only metric would
  read zero on the build that shows the holes.
- Q3. Ticket 01 adds GPU attribution. frame-profiling.md blocks GPU and per-view
  optimizations on it, and frame p50 (8.4 ms) is well above main (3.2 ms) and
  render CPU (3.6 ms). The render tickets are blocked by it.
- Q4. Targets come from the display: stationary and flight p95 at most 8.3 ms;
  both sprints p95 at most 16.7 ms with no frame over 33 ms; 6.9 ms (144 Hz)
  stationary is a stretch, not a gate. Fluid's Server share is reported
  separately. The flat demo's 6.2 ms is context only: it drew less and had no
  lakes.
- Q5. The coarse generation ticket profiles first. The shared pass saves the
  34-squared over 32-squared 2D overlap, about 13%, and no 3D cave samples, so
  it is built only if the profile shows a measurable share. Direct generation
  at each level's own resolution joins the column-only level as a probe.
- Q6. Ticket 01 records worker CPU per stage and level. A level-0 job-cost
  ticket triggers if level-0 jobs exceed 50% of worker time in a sprint.

## Since the interview

- PR 7 merged as #30 (`446e984`) and this branch was rebased onto it. Round 1
  Q12 and round 3 Q4 resolve: the baseline uses `main`'s demo, pinned by hash,
  and ticket 04 is no longer blocked.
- The primary-source research landed ([research.md](research.md)). It confirms
  the coverage rule (Voxy, Distant Horizons) and the score's shape (Sodium,
  Luanti), and raised two questions, both accepted:
  - Q1. Ticket 08 stays at 16 B a vertex inside Bevy's mesh path. Per-quad
    packing (Voxy's 8 B a quad) is a follow-up only if upload or vertex cost
    still dominates after it.
  - Q2. Ticket 12 also measures level-0 mesh time spent on Chunks a Server
    walk from the Eye cannot reach. Over 30% in a sprint, they move to the
    lowest mesh priority; never skipped.

## Round 5: review of the written plan

An independent review of the spec and tickets found two blocking problems,
mechanical errors (demo settings are 160 m view and 3000 m LOD reach, not the
defaults the spec quoted; ticket 04's border saving exists at level 1 only;
empty meshes spawn no entity) and missing fixtures, columns and definitions,
all fixed in the tickets. Four decisions, all accepted:

- Q1. Holes are counted as two kinds. Rule holes (delivered or retained, but
  not drawn) gate 02 and 07 at zero in every fixture; fill holes (nothing
  delivered) are zero in the measured phase of the dive and both sprints and
  gate 03 and 11.
- Q2. A level-k fragment is discarded wherever any finer level has delivered
  the Chunk at that point, revising round 2 Q2: checking level k-1 alone left a
  retained level 2 drawing over delivered level 0, since level 1 never meshes
  inside its own hole. "All eight children covered" only releases retained
  meshes.
- Q3. The Server's view cone is a fixed 60 degree half-angle with its apex 6 m
  behind the Eye; the Client still sends nothing new.
- Q4. Moving the mesh attribute clone off the main thread is its own ticket,
  14, blocked by 01 only; ticket 09 is batching alone, blocked by 08. Ticket 15
  checks the spec's targets once everything else is closed.
