# 08: Keep fluid and fire changes visible through LOD

Status: done
Blocked by: 06, 07

Spec: [decisions 7 and 11](../spec.md).

## What to build

Keep full-detail and held level-1 views responsive to flowing, waterlogging,
reactions, burning, and extinguishing, without moving dynamic water to the
generated waterline or simulating distant terrain.

## Acceptance criteria

- [x] Auxiliary fluid/burning changes invalidate level-1 covers even when
  the packed solid Voxel is unchanged; eviction removes held changes under
  ADR-0011's existing semantics.
- [x] Level 1 derives a documented volume-based approximation of held changed
  state. Coarse levels never simulate fluids, exposure, or fire.
- [x] Higher levels retain generator-only content and coarse-light limits;
  document the resulting distance discontinuities.
- [x] Generated-waterline corrections keep their existing lake seam behavior
  where applicable and never relocate arbitrary waterfall/waterlogged surfaces.
- [x] Ring masking continues to avoid double coverage or holes for both
  generated and dynamically changed water at supported Voxel sizes.
- [x] Ongoing changes do not starve full-detail or level-1 mesh delivery.
  Coalesce invalidation and retain follow-up work after an older mesh lands.
- [x] Light Snapshot validity includes fluid, fire, and geometry changes;
  both brightening and darkening agree with a fresh solve under the explicitly
  shared per-Voxel light contract.
- [x] Held/scrambled Jobs and controlled-tick integration tests verify stale
  result handling, boundary changes, mesh redelivery, and eventual settling.

## Verification and completion

Run relevant LOD, meshing, light, and Server integration tests and comparisons.
Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use the required Conventional Commit and trailer.

## Comments

- Per-void light storage and a separate coarse emitter flood are not added.
  This ticket integrates the declared approximation, rather than expanding
  thermal/fluid simulation to distant LOD rings.
- 2026-10-23: done. The commit that closes this ticket is the one this
  comment lands in.

### Implementation

- `tesserae-core/src/lod.rs`: `downsample_changed(children, coord_y, tables,
  waterline) -> Coarse { chunk, overlay }` derives level 1. A child is
  `Child::Packed` (generated, or held and never changed: the old path,
  unchanged) or `Child::Changed(chunk, overlay)`, read through `Occupants`.
  The rule, documented on the function and in ADR-0011: a fluid weighs what it
  fills (its surface's share of the cube in air; `Void::volume_below` of each
  wet void in a Shape) and competes with solids by volume exactly as generated
  water does. Packed water below the `Waterline` (what generation could have
  made) is lake water and stays packed. Any other fluid is dynamic: a coarse
  Voxel whose fluid is mostly dynamic is air with flowing fluid in the coarse
  overlay at the share of the coarse Voxel its contents fill, at least 1/16.
  A coarse solid burns when half its Material's volume does. Nothing is
  simulated. `Tally` is const-generic (8 for packed children, 16 where a fluid
  can stand beside a solid). `Worldgen::voxel_size`.
- `server/lod.rs`: the generate stage returns `Coarse`; `LodStore` keeps level
  1's non-empty overlays beside its Chunks (`shown`). Dispatch hands each held
  level-0 child with its overlay exactly when `MeshState::is_edited` (no
  longer test-only), so an unchanged child costs what it did. Levels >= 2 are
  untouched: generator-only, no overlay. Mesh jobs read the centre's and the
  six neighbours' overlays (`Shown`), so coarse water culls across coarse
  Chunk faces and coarse fire draws flames (pass 5). Fluid in the overlay is
  lit as the air it stands in; no coarse emitter light.
- `level_the_water` moves only upward water quads over packed (lake) cells. A
  quad over lake and overlay cells is split into per-cell quads (corner
  order, light and UVs kept) and only the lake's cells move.
- Invalidation: confirmed, no change needed. `set_occupants` calls
  `voxels_changed` for every change, overlay-only ones included, and
  eviction of an edited Chunk marks its cover; tests below pin both for a
  fluid-only change.
- Light Snapshot validity (`streaming.rs`, `world_state.rs`, `edit.rs`):
  `MeshState::edited` now spoils only its own Chunk's running solve.
  `LightLattice` records `across`, the Chunks across a face whose plane of
  light the relight wrote, and `set_occupants` marks those edited (dirty and
  spoilt). A bulk solve reads its Chunk and only the facing plane of each
  neighbour, so this is what it depends on. Before, every change spoilt all
  six neighbours' solves: water changing anywhere in a Chunk kept the solve
  next door, and every mesh waiting on its light (`wait_for_settled_light`),
  from ever landing. It also fixes a stale mesh: a neighbour whose faces a
  moved plane lights, when none of its own light moved, was never redrawn.
- Mesher: pass 1 hides a full fluid cube's top under a partial surface of the
  same Material (an inner face, at level 0 as at level 1), in its own
  `pass1::<true, true>` instance, since the test cost 4-7% on translucent
  Chunks with no surfaces. Connected Shapes read their arms from the packed
  Voxel (`Neighborhood::packed_at`): full flowing water no longer grows a
  fence an arm that collision and the kernel lack (ticket 04's note).
- Docs: ADR-0011 (fluid and fire at level 1, invalidation, the distance step,
  emitters, only the lake meets the waterline), CONTEXT.md's Downsample.
- Bench: `lod/downsample/changed_dry` and `changed_flooded`.

### Verification

- Test functions: core lib 314 (309), meshing 66 (64), Mods 89 unchanged
  (three ignored), `tesserae` binary 341 (331, one ignored as before);
  workspace 810 (793). Full `scripts/cargo-ci` passed on the tree this commit
  records.
- Core: a changed child with an empty overlay gives `downsample`'s result
  (random children and real lake shores); dynamic fluid stands at its filled
  share (full, half, a 1/16 film, stone winning, stone dropped under half);
  only water below the waterline stays packed, mostly-lake cells stay lake, a
  tie is the lake's; waterlogged volume is the voids' (slabs, stair notch);
  a coarse solid burns at half. Meshing: no face between full water and a
  lower surface over it (and glass under water keeps its top); no fence arm
  towards flowing water, an arm towards a source.
- Level 1 (`lod/tests/held.rs`, LOD harness): flowing water, a film and a
  fire set through `set_occupants` reach the cover exactly as an independent
  `downsample_changed` gives, the cover and its neighbours' meshes are
  redelivered equal to fresh meshes, with water and flame quads, and no
  level 2-3 Chunk is rebuilt; a fluid-only change (packed Voxels unchanged)
  twice regenerates the cover and redelivers it, and eviction returns the
  cover to the generator's with an empty overlay; water changing every tick
  with the generate stage held then scrambled (and meshes scrambled) gets
  at least two regenerations and two meshes delivered while it changes, then
  settles to the expected cover and a fresh mesh; through the controlled tick
  a source placed on a floor flows by the simulation's own landings and the
  settled cover and held mesh match. Waterline (1.0, 0.5, 0.25 m): the lake
  is moved to level 0's surface, flow one Voxel over it beside it is not,
  their shared quad is split, each covered once, a fall's top is not moved.
  Ring masking (1.0, 0.5, 0.25 m): every blended face of water that moved,
  sampled behind as the shader samples it, lies in a coarse Voxel whose
  level-0 cells hold water, across a level-0 Chunk floor inside the level-1
  Chunk (pool with a film above the floor, film on stone, a fall through it,
  waterlogged slabs); the film shows in the upper Chunk.
- Level 0 (`streaming/tests/progress.rs`): a neighbour's held solve lands
  beside water toggling every tick deep in the next Chunk, and that Chunk is
  meshed meanwhile, with no stale solve; a held solve over water arriving and
  leaving and a fire lit and put out on the plane it read is dropped each
  time, and the light equals a fresh `light_cells` sweep after each (darkening
  and brightening); a lantern in a sealed tunnel lights the face of the stone
  across a boundary, whose Chunk is redrawn though none of its light moved;
  water toggling on a boundary every tick with meshes held then scrambled
  delivers both Chunks, at least one mesh lands behind a change, and after
  settling both are fresh.
- Mutation checks, each failing exactly the tests named: the old
  six-neighbour spoiling (the toggling-water solve test, at the 60 s ceiling);
  not marking `across` (the tunnel test); correcting the waterline over
  overlay cells (waterline test); level 1 never given overlays (the three
  harness tests of held fluid); arms from `at` (fence test); before the pass
  1 rule, the fall's inner face failed the waterline test.
- No Client mutator; core gained no dependency; `Cargo.lock` unchanged.

### Hot-path comparison

Ticket 01's method: pre-change bench binaries from `8c43deb`, post from this
tree, both with the uncommitted thread-CPU-time criterion patch, interleaved
(pre first in odd rounds), `--warm-up-time 1 --measurement-time 2`, uptime
before and after every process, in `/private/tmp/tesserae-p8-t08.a/` (not
committed). LOD and meshing, three rounds, 22:18-22:53, load 3.2-7.6 (round
2's pre lod at 6.9-7.6, which moves no conclusion). Meshing again after the
pass-1 instance, three rounds, 23:06-23:15, load 2.6-4.0. Figures are the
minimum over rounds of criterion's median, and the change in the medians:

| bench | pre | post | min | median |
| --- | ---: | ---: | ---: | ---: |
| lod_downsample/worldgen | 255.7 us | 206.7 us | -19.2% | -19.1% |
| lod_downsample/testgen | 289.0 us | 233.4 us | -19.2% | -19.4% |
| lod_downsample/stairs | 593.3 us | 489.9 us | -17.4% | -17.6% |
| lod_downsample/stair_surface | 329.7 us | 271.0 us | -17.8% | -22.6% |
| lod_coarse_chunk/surface_level_1 | 2.13 ms | 2.00 ms | -6.0% | -6.0% |
| lod_coarse_chunk/surface_level_5 | 776.7 us | 720.6 us | -7.2% | -6.9% |
| lod_mesh_coarse/surface_level_1 | 436.8 us | 434.4 us | -0.5% | -1.2% |
| lod_mesh_coarse_lit/surface_level_5 | 447.9 us | 455.8 us | +1.7% | +2.9% |
| greedy_mesh/glass_room | 294.9 us | 297.3 us | +0.8% | -0.4% |
| greedy_mesh/terrain_water | 444.3 us | 450.7 us | +1.4% | +0.4% |
| greedy_mesh/terrain_canopy | 463.4 us | 469.0 us | +1.2% | +1.4% |
| mesh_chunk/shore | 463.7 us | 467.3 us | +0.8% | +0.7% |
| mesh_chunk/declined_air | 21.7 ns | 22.6 ns | +4.1% | +3.9% |

- Every other LOD row moved -1.1% to +1.2% (coarse light, coarse chunk air
  and rock, coarse meshes), every other meshing row -1.9% to +1.2%. The
  downsample rule is unchanged; the gain came with splitting
  `downsample_general` per octant and the const-generic `Tally`, and is not
  investigated further. The two declined meshes are a 0.9 ns early-out this ticket
  does not touch: code placement. The first post build put glass_room at
  +7%, which a bisection traced to the pass-1 surface test and the separate
  instance removed.
- New, post only: `lod_downsample/changed_dry` 718 us (eight changed dry
  children read through `Occupants`, 3.5x the packed path, paid only by
  edited children) and `changed_flooded` 1.20 ms.

### Deviations, limits and what later tickets inherit

- The approximation is a mip. A one-Voxel stream sharing a coarse Voxel with
  its floor loses it to the stone; on an odd row it shows (the controlled-tick
  fixture is built that way, commented). Waterlogged water mostly loses to its
  Shape. Coarse fluid in the overlay is lit as air, so dynamic water under the
  terrain is dark and in the open is not dimmed by depth, and lava and fire
  emit nothing at level 1, per ADR-0011.
- Packed sources placed below the waterline are lake water to level 1: the
  canonical encoding cannot tell them from generated water, and they are
  corrected to the waterline like it. Above the waterline, or any fluid other
  than water, is dynamic.
- Ring masking: each coarse water face lies in the coarse Voxel holding the
  water it draws, so the mask's per-Chunk choice is the right Chunk's. One
  coarse surface per coarse column cannot match four level-0 columns ending
  on both sides of a level-0 Chunk floor; ADR-0011 records it.
- Distance step: held fluid and fire show at level 1 only; from level 2's
  inner edge (256 m at the defaults) the generator's water returns and fire
  is gone, documented in ADR-0011. Coarse levels never simulate.
- Mesh progress: an older mesh is delivered while a newer one waits at both
  levels, as before, and is now pinned for fluid changes. What can still hold
  a mesh back: a writer changing the very plane a neighbour's bulk solve
  read, every tick for ever, spoils that solve each time, as correctness
  requires, and `wait_for_settled_light` then holds the neighbour's mesh and
  its neighbours'. Flow parks changes in an unlit Chunk only, not beside one.
  Finite flow and fire end; a Mod toggling a source on a boundary beside a
  newly arrived Chunk would not. For ticket 09 to weigh.
- For ticket 09: `downsample_changed` per edited cover (above);
  `level_the_water`'s split is per affected quad and rare; `across` adds a
  few comparisons per relit Voxel.
- For ticket 10: `Coarse`, `Child`, `Waterline`, lake versus dynamic water at
  level 1 are new nouns for CONTEXT.md. No live capture was taken; level-1
  output is pinned by mesh equality and quad tests.

