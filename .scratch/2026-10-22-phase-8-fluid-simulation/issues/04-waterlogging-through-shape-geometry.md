# 04: Waterlog partial Shapes without leaks

Status: done
Blocked by: 03

Spec: [decisions 3, 4, 5 and 11](../spec.md).

## What to build

Let water occupy and flow through accessible empty space around a solid
Shape, with clipped visible surfaces and unchanged solid collision/selection.

## Acceptance criteria

- [x] Slabs, stairs, posts, fences, and declared nodeboxes can coexist with
  compatible water without changing solid Material/Shape/Orientation.
- [x] Passage derives from the existing sixteenth-resolution Shape geometry
  at registration, including Orientation and actual derived connection arms.
- [x] Tests prevent flow through sealed faces, jointly sealed interfaces,
  internal partitions, and disconnected voids. Sealed cavities do not become
  wet merely because another void in the same Voxel is wet.
- [x] Only one fluid Material can occupy a Voxel, with region state where
  necessary to preserve reachability. Arbitrary non-solid Materials are not
  implicitly replaced by water.
- [x] Partial solid placement preserves compatible fluid; full solid
  placement displaces it; solid removal preserves remaining fluid. Source
  versus flowing identity survives these operations.
- [x] Source placement/removal targets waterlogged state without deleting its
  solid. Invalid or cap-exceeding operations remain atomic.
- [x] A half-metre gravity drop within partial geometry contributes to the
  physical reset-drop rule at different Voxel sizes.
- [x] Meshes clip water to accessible voids without changing collision or
  selection. Connection changes wake passage and remesh affected boundaries.
- [x] Light increases/decreases and stale Snapshots remain correct under the
  explicitly shared per-Voxel light approximation. Precise per-void light is
  not silently added.
- [x] Dry/full-cube paths retain their cheap gates; topology does not become
  a per-tick full 16-cubed simulation for every terrain Voxel.

## Verification and completion

Run relevant native geometry/meshing/light tests and controlled Server tests.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- The proposed waterlogging model preserves disconnected geometry for passage
  while retaining the existing light-storage resolution.
- 2026-10-23: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- `shape.rs`: every variant (Shape x Orientation, and each arm submask of a
  connected Shape) derives its `Void`s at registration from the rotated
  sixteenth grid: 6-connected empty regions that reach a face, numbered by
  their lowest sub-cell. A void has its open squares per face (`FaceMask`
  layout), its floor, its sub-cells, and its full-surface quads. A cavity
  that reaches no face is no void. Air is one whole-cell void, a cube none.
  `MAX_VOIDS` is 4: a Shape with more, under any arms for a connected one,
  holds no fluid at all (closed, never merged). `ShapeTable::voids(voxel,
  arms)`, `void_slots`, `Void::quads(surface)` (boundary squares below the
  surface plus the cut surface where the void goes on above it),
  `Void::overlaps`, `FaceMask::intersects`/`union`.
- `occupants.rs`: `Fluid` keeps `surface`/`reach`/`descent` as its first
  void's `Level` and adds `others: [Level; 3]`, so separate voids keep
  separate state (one shared state would let a wet void lend reach across a
  partition, and with a shelf would let a source-less loop sustain itself).
  A source fills every void and keeps `others` dry. `Fluid::flowing`,
  `level`, `levels`. `Overlay` is 30 bytes (12 before).
  `place_solid(voxel, &Tables)`: a solid with voids keeps the fluid, a source
  as a source and flow by sub-cell overlap (a level at or below the new
  void's floor is dropped); a solid with none displaces it; breaking keeps
  it. `Tables::vet_occupants` refuses fluid where `void_slots` is 0, flow wet
  in a void the solid lacks, dry levels carrying state, and flow dry
  everywhere.
- `flow.rs`: `wanted` evaluates each void of a Voxel. Offers pass between
  voids whose open squares meet across the shared face; nothing enters
  sideways from a void with a lower floor; descent adds the sixteenths fallen
  between floors (`16 + floor above - floor below` falling, the floor
  difference sideways), renewing reach at the reset drop. A dry Voxel's one
  Material is the best offer into any void; delay is the shortest over the
  voids that change. A connected Shape's voids come from its actual arms,
  read from the packed Voxels across its faces (unknown pauses). `reads(
  connected)` widens `READS` to those faces' solids when any connected Shape
  is registered; `dependents`, `readers_outside(chunk, connected)` and
  `reading_outside(chunk, connected)` (faces two deep then) follow from it.
  `wake_solid_around` wakes the readers of a changed solid and of each
  connected Shape beside it. For empty space every rule reduces to ticket
  02's, and every earlier kernel test passes unchanged.
- Light: `Tables::wet_light_params`/`wet_emission` are the shared per-Voxel
  approximation: the solid's seals, the larger of solid and fluid dimming,
  the brighter emission, one value whatever the voids. `LightVolume::wet`
  (default none), asked only of shaped Voxels, feeds `relight`; the fresh
  solve is the new `light_cells(chunk, overlay, ..)`, which `jobs::solve`
  now uses. Both are documented as the same approximation. No per-void
  light.
- Meshing: `Neighborhood` collects fluid in solids; pass 4 draws each wet
  void's `Void::quads` clipped to its surface, culling a boundary quad where
  the Voxel across covers it with its solid or with the same fluid filling
  its voids, and pass 1 and 3 hide water faces against a waterlogged Voxel
  whose solid and full voids cover the face. The solid's own pass-2 geometry,
  collision and selection read only the packed Voxel and are unchanged.
- Server: `set_occupants` relights when the optical Voxel or the fluid in a
  shaped solid changes and journals whether the solid changed
  (`Changed.solids`); `fluids.rs` wakes connected Shapes beside a changed
  solid, uses `reads(connected)` for Snapshots and staleness, and the
  widened admission spans. The fluid tool places a source into a struck
  Shape that has voids (else in front, as before), refusing one already
  holding a source or another fluid; removal already kept the solid.

### Verification

- Core: 266 lib tests (252 before), meshing 63 (60). Server: 308 in the
  `tesserae` binary (304), one ignored as before. Mods 74, unchanged.
  Workspace 711 (690). Full `scripts/cargo-ci` passed on the tree this commit
  records.
- Shape: for slab, stair and post under all 24 Orientations the void is
  exactly the empty sub-cells and its open squares the complement of the
  coverage; a hollow box's cavity is no void and draws nothing; a wall makes
  two disjoint voids; five voids hold no fluid; a stair's notch draws a full
  and a clipped surface with the expected areas.
- Kernel (`flow::tests::waterlogging`): a stair fills from above, keeps its
  solid, is not climbed into from the floor, and dries without supply; a
  stair's sealed back and a bottom slab beside a top slab pass nothing while
  the open turn and two bottom slabs do; a partition keeps its far void and
  beyond dry; a shelf's two voids hold different reach (5 and 6) and a loop
  through them dries; half a metre of descent inside slabs renews reach at
  1.0, 0.5 and 0.25 m and a quarter metre does not; a connected arm plugging
  a window closes, opens and closes passage as its neighbour comes and goes;
  quota 3 gives the same state at all 300 steps. The read-span test now
  checks the narrow and wide sets Voxel by Voxel.
- Tables: per-void vetting; the wet light combination.
- Light: waterlogged stone stairs between a lantern and a room dim it and
  draining restores it exactly, the incremental path equal to `light_cells`
  both ways.
- Meshing: a waterlogged stair draws four quads, all in its notch, and its
  opaque mesh is the dry one's; a waterlogged slab's water stands from 8/16
  to its surface; a waterlogged stair in a lake leaves only the lake's six
  outer faces.
- Server: the fluid tool waterlogs a struck stair, is refused on it again
  (Occupied, nothing changed), and drains it, the packed Voxel a stair
  throughout; a stair into a source keeps it, breaking leaves the source, a
  cube displaces it, a stair refused at the shaped cap leaves the water as
  it was. In the controlled tick, a stair waterlogged by the tool feeds the
  corridor towards the lantern and not through its sealed back, the light
  dims and returns, equal to a fresh `light_cells` each time, and the stair
  stays. In the streaming harness a waterlogged fence is remeshed equal to a
  fresh mesh after its source arrives and after a stone beside it grows an
  arm.
- Mutation checks, each failing the tests named: ignoring open-square
  intersection (shelf, partition, arm, sealed faces); allowing climbing
  (stair, shelf, division, sealed faces); voids without arms (arm); a fall
  that ignores floors (half-metre); relight ignoring wet solids (light);
  `wake_solid_around` not waking connected neighbours (arm). Not pinned at
  the Server: its own connected-neighbour wake (`wake_around_solid`); the
  dev table's fence cannot build a Voxel whose far reader is out of the
  narrow set, and the logic is the kernel's one, which is pinned.
- No Client mutator; core gained no dependency; `Cargo.lock` unchanged.

### Deviations, limits and what later tickets inherit

- Flow never climbs within geometry either: water on a floor does not run up
  into a slab's or stair's void beside it. Voids fill from above, from voids
  at their floor or higher, or by the fluid tool. A lip (a void whose
  opening is above its floor) is not modelled: passage reads open squares
  and floors only.
- `MAX_VOIDS` = 4 caps what a Shape may hold; past it the Shape excludes
  fluid. Ticket 05's declared waterlogging eligibility can only narrow this.
- Flow moved by a change of Shape is mapped without arms; a connected
  Shape's flow is derived again from its arms when next evaluated, and
  levels stored for voids that a change of arms removed are cleaned up by
  that evaluation.
- The mesher computes arms from its padded cells, where full-surface flow
  reads as a water cube, so a `to_sealed` Shape beside full flowing water may
  draw an arm the kernel and collision do not have (ticket 02's shell, not
  changed here). Shell Voxels' voids are read without arms for culling, and
  a water quad against a lower partial surface beside it is drawn whole
  (overdraw only).
- For ticket 05: `Fluid::others`/`Level`, `ShapeTable::voids`, the
  per-void rules and `light_cells` are what declared fluids flow through;
  lava's opacity combines by the same maximum (dimming 15 in a stair stops
  entering light, its emission leaves through the open faces).
- For ticket 07: heat passage should read `voids`/`open`, never the light
  approximation; whole-Voxel wet fuel can test `Fluid::levels`.
- For ticket 08: level 1 still reads packed Voxels; waterlogged water is
  absent from it.
- For ticket 09: unmeasured: registration now flood-fills every variant
  (`shape_register_*`, `shape_table_builtin`), each evaluation looks up
  voids for its six reads, the overlay entry grew to 30 bytes, and with a
  connected Shape registered (the dev build) admission rescans faces two
  deep. No bench was rerun.
- For ticket 10: `Void` and waterlogging are new nouns for CONTEXT.md.
