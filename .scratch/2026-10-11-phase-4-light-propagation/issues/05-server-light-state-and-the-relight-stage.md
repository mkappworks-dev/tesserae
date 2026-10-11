# Server light state and the relight stage

Status: done
Blocked by: 02, 03, 04

Light is World state, so the Server owns it outright and the Client gets it
already baked into the vertex attribute. Spec decision 7.

## What to build

`server/world_state.rs`:

- `ChunkStore` gains a second map of `ChunkLight`, with its mutators
  `pub(in crate::server)` exactly like `chunks`. That privacy is the whole
  enforcement of ADR-0002 - a Client system can ask for `ResMut<ChunkStore>`
  and find nothing it can call.
- A `pub(in crate::server) fn set_voxel`, which `relight` and ticket 07 need.
- The `LightVolume` impl from ticket 03, over the two maps.

`server/streaming.rs`:

- A `light_dirty: HashSet<ChunkCoord>` on `MeshState`.
- A `relight` stage between `generate` and `publish`, with
  `relight_per_tick` on `StreamingBudget`.
- A second offset order on `LoadOrder`: the load extent sorted by **descending
  y, then by distance**. Sky light flows downwards, so solving a Chunk before
  the one above it means solving it twice; nearest-first is right for
  generation and wrong here.
- The open-sky boundary. For a Chunk whose +Y neighbour is not loaded, pass a
  synthetic uniform sky-15 `ChunkLight` when that neighbour's floor in metres
  is above `Worldgen::surface_bounds_m().1`, and `None` otherwise. Above the
  band the generator makes air everywhere, so the test is purely vertical and
  exact. The `Uniform` variant allocates nothing, so building it per call is
  free.
- A relight that changes a Chunk's light marks it mesh-dirty and marks its six
  neighbours light-dirty.
- `publish` additionally requires the centre's and all six neighbours' light to
  be present.

## The comment in `generate` that this ticket invalidates

`generate` currently argues at length that a neighbour arriving can never
invalidate a mesh the Client already holds, and names
`no_mesh_the_client_still_holds_is_stale` as the test that will fail when that
stops being true. **It stops being true here**, and not for the reason the
comment predicted - not Voxel edits, but light crossing Chunk boundaries, so a
Chunk two Cells away changes the light in between and so changes ours. Rewrite
the comment to say what is now true: a mesh is redelivered when light changes
it, and the Client's replace-in-place upload path already handles that.

## Why the metre test and not a Voxel-count test

`surface_bounds_m` is in metres and so is the Chunk floor. ADR-0003: a
comparison in Voxel indices would be calibrated to one Voxel size.

## Tests

- A tick with no anchor still does nothing.
- One tick generates, relights and publishes, and the published mesh carries
  non-zero light on the top faces of surface Chunks.
- The open-sky boundary fires: a Chunk at the top of the load extent above the
  band gets sky 15, and one below the band gets 0.
- A relight that changes light re-publishes the Chunk; one that changes nothing
  does not.
- `relight_per_tick` caps the work per tick.
- Re-point `no_mesh_the_client_still_holds_is_stale` at what is now true.

## Comments

Two things only measurement found.

**The relight queue did not drain.** Marking all six neighbours for a re-solve
whenever a Chunk's light changed meant that during a fill — where every
Chunk's light changes at least once — each Chunk was queued about seven
times. The
queue grew monotonically (20 to 850 and climbing) and the app spent the whole
fill in its heaviest regime. The fix is `ChunkLight::boundary_differs`: a
neighbour only needs re-solving if the plane it actually reads moved. Comparing
1024 cells costs about 6 us against the 131 us re-solve and 386 us re-mesh it
avoids.

**The budget went the other way from the obvious one.** `relight_per_tick` is
12 while `generate_per_tick` is 24, which reads backwards — every Chunk that
arrives needs its light. It is right because relight is the one stage whose
backlog is invisible: a Chunk is not published until its light is solved, so
what queues up behind it is the Chunks out past the mesh extent that nothing
draws. At 24 the worst frame during the first fill ranged 65-115 ms over
three runs; at 12, 39-48 ms; the fill finished inside 2 s either way.

A third attempt failed and is worth recording so nobody repeats it: gating
relight on the Chunk above being solved, so sky light would settle in one
top-down pass, made the worst frame *worse* (68 to 103 ms). The loop spent its
time scanning past blocked Chunks instead of doing work.
