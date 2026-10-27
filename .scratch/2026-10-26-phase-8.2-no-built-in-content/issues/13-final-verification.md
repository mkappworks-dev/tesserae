# 13: Final verification

Status: done
Blocked by: 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12

Spec: [all decisions, and the Testing Decisions](../spec.md).

## What to build

Check the step as a whole before the PR.

- An interleaved thread-CPU A/B of the meshing, collide, LOD and fluids
  benches against main, as step 8.1's ticket 10 ran it. Every fixture id
  moves from 1 to 6 to 8 and up, and step 8.1 saw +7.5% on
  `greedy_mesh/terrain_water` from an id move alone, so a row that moves is
  investigated, not waved through.
- A `cargo dev` walk-through with captures in one `/tmp` folder: terrain, the
  gallery, the menu, the bar, the outline, a ghost and the corner model, and
  a World with no Game.
- Docs: DESIGN.md's step 8.2 entry is complete, and every doc the spec lists
  matches the code.
- An inline, copyable PR body following the template, concise bullets, no
  footer.

## Acceptance criteria

- [x] The A/B shows no change beyond noise, or each change is explained in the
      Comments.
- [x] The phase 7 drain cap and the Wasmtime, WASI and Cranelift pins are
      unchanged.
- [x] The user has reviewed the captures, and the `/tmp` folder is deleted
      afterwards.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass on the
      final tree.

## Comments

### 2026-10-26: final review and checks

At the user's request, tickets 01 through 12 were committed before running
formatting and full CI. Their individual commits were not independently
validated; the completed tree is the validation target. Final review fixed
fixture ids aliasing loaded Game ids, tests retaining the old remove-tool
semantics, and the guest builder's shared-target artifact lookup. The
host-call bench has a separate declaring 0.1 bench-builder guest; frozen
builder and echo sources and their 0.1 WIT remain unchanged. The naive
benchmark-only contract receives the registered material id explicitly.

Ran cargo fmt --all, the standalone changed guests' formatters, the demo
formatter and npm run fmt. ./scripts/cargo-ci passed on the final Rust tree:
875 tests passed (382 app, 327 core, 69 meshing integration, 97 Mods), zero
failures, five existing manual measurement tests ignored. Clippy, rustdoc,
default-feature build, guest/demo rebuild, benchmark compilation, boundary
checks and cargo-deny advisories/licenses/bans/sources passed. No dependency
pins or lockfile changed; MOD_VOXELS_PER_TICK remains 2.

Review added coverage for all retired names and ids, outside-item F cycling
with orientation preserved, ghost rejection while lighting is unsettled,
fire outlining fuel without a ghost, and stale/menu-hidden outlines. The
menu preserves a dragged Shape tile until the drag ends. Captures prompted
label wrapping, ghost shadow suppression and a smaller shaded corner model.
The user requested two further reductions of that corner model; its final
scale is 0.06. The user also replaced the flat-colour tile decision with
model previews in the menu and hotbar. A shared model atlas uses the same
geometry, preserves orientation, deduplicates repeated items and releases
replaced assets. Headless tests cover that lifecycle. The offscreen capture
keeps the atlas camera separate from the captured World cameras. Both menu
and previews remain engine Client UI; the Game supplies content and the
Server answers aim and placement.

Captures use the dev binary with the default demo and the offscreen capture
path, in /tmp/tesserae-p82-review-569fb3d6: terrain/gallery/bar, menu, two
stair orientations with outline/ghost/corner model, and an empty no-Game
World. The capture driver cannot click; input and drag behaviour are covered
by headless tests. The user reviewed and accepted the final model tiles and
smaller corner model on 2026-10-26. The review folder was then deleted.

### Benchmark method and exceptions

Baseline is origin/main at 212d9008f33730ed2250ff620bb2fafced80284b,
the phase 8.1 merge. The local main ref still points to the older 9992e8e,
so using it would compare against the wrong phase. No fetch or push ran.
Core-only temporary workspaces contain the baseline and candidate source,
with separate Cargo targets and copied executables. Instrumentation uses
CLOCK_THREAD_CPUTIME_ID, foreground thread QoS, caffeinate, alternating
main/post order, 0.2 s warmup, 0.5 s measurement and 20 requested samples
(the existing per-group sample counts remain). Raw artifacts are under
/tmp/tesserae-82-ab-546439f142 and are not committed.

An initial shared-target build reused one executable for both snapshots;
hashes exposed this before accepting results. Those measurements were
excluded, and targets were separated and hashes checked. The first two
corrected rounds overlap the user's requested preview rebuild, captures
and CI and are also excluded. Automatic approval review could not stop the
benchmark process because its review service hit a usage limit, rather
than judging the action unsafe. The process was allowed to finish; clean
rounds are evaluated separately.

Four clean full rounds (3 through 6), 115 rows. Three longer targeted
rounds use 0.3 s warmup, 1 s measurement and 30 requested samples.
The same-binary id probe also ran three times. Sampled one-minute load
during the later broad runs was about 1.6 to 2.0. No render or build ran
during the accepted rounds. Rows beyond 3% on minimum or median follow.

- `collide/fall_slabs`:
  main/post minima 41.75 ns / 40.12 ns; minimum -3.9%, median -4.0%.
- `fluids_saturated/forget_one_chunk_of_full_agenda`:
  main/post minima 151.16 us / 130.87 us; minimum -13.4%, median -11.0%.
- `greedy_mesh/all_stairs`:
  main/post minima 4.163 ms / 3.462 ms; minimum -16.8%, median -17.5%.
- `greedy_mesh/terrain_stairs_at_budget`:
  main/post minima 368.68 us / 355.27 us; minimum -3.6%, median -3.1%.
- `greedy_mesh/terrain_stairs_at_cap`:
  main/post minima 496.97 us / 477.35 us; minimum -3.9%, median -4.4%.
- `light_chunk/leaves_canopy`:
  main/post minima 167.10 us / 173.41 us; minimum +3.8%, median +1.4%.
- `light_chunk/opaque`:
  main/post minima 4.05 ns / 4.24 ns; minimum +4.6%, median +3.6%.
- `light_chunk/open_sky`:
  main/post minima 4.06 ns / 3.90 ns; minimum -3.8%, median -3.9%.
- `lod_coarse_chunk/rock_level_1`:
  main/post minima 66.01 ns / 68.12 ns; minimum +3.2%, median +4.4%.
- `lod_coarse_light/surface_level_5`:
  main/post minima 107.39 us / 104.21 us; minimum -3.0%, median -3.0%.
- `lod_downsample/changed_dry`:
  main/post minima 695.35 us / 728.28 us; minimum +4.7%, median +3.4%.
- `lod_downsample/changed_flooded`:
  main/post minima 1.201 ms / 1.227 ms; minimum +2.2%, median +3.1%.
- `lod_downsample/uniform_stone`:
  main/post minima 3.19 ns / 3.44 ns; minimum +8.0%, median +8.0%.
- `mesh_chunk/cave_mouth`:
  main/post minima 248.59 us / 236.76 us; minimum -4.8%, median -4.7%.
- `mesh_chunk/declined_enclosed`:
  main/post minima 12.89 ns / 13.46 ns; minimum +4.4%, median +3.7%.
- `mesh_chunk/ramp_densest`:
  main/post minima 335.96 us / 320.26 us; minimum -4.7%, median -4.6%.
- `snapshot/arc_clone_seven_chunks_and_lights`:
  main/post minima 34.79 ns / 30.04 ns; minimum -13.6%, median -5.7%.
- `snapshot/clone_seven_chunks_and_lights`:
  main/post minima 4.52 us / 4.55 us; minimum +0.7%, median -4.8%.
- `snapshot/make_mut_shared`:
  main/post minima 303.84 ns / 289.07 ns; minimum -4.9%, median -3.9%.
- `snapshot/make_mut_shared_light`:
  main/post minima 431.43 ns / 479.26 ns; minimum +11.1%, median +14.1%.
- `worldgen/cave_band_uniform`:
  main/post minima 30.10 ns / 31.17 ns; minimum +3.6%, median +3.6%.

The mesher's existing algorithms, MaterialTable::def, collision routines,
LOD downsampling, snapshot operations and Agenda are unchanged. The mesh
change is the separate isolated-Shape helper for Client previews. The
stair-heavy improvements reproduce in the longer targeted runs, so they
are recorded rather than presented as a new optimisation.

A separate relinked baseline probe copies the six Material rows, remaps
Chunk ids 1..6 to 8..13 and water 8 to 14, and asserts identical positions
in every mesh layer before measuring. In one executable, three longer
rounds put old/moved median times at:

- terrain: 217.47 / 216.90 us (-0.3%);
- solid: 214.97 / 216.62 us (+0.8%);
- water: 285.54 / 284.78 us (-0.3%);
- canopy: 308.90 / 309.35 us (+0.1%);
- stairs at cap: 487.35 / 487.67 us (+0.1%);
- all stairs: 3.471 / 3.501 ms (+0.9%).

Old ids in this probe cost 3.471 ms for all stairs, versus 4.202 ms in the
original baseline executable, although the algorithm and input are the
same. That supports code layout as the cause of the large cross-binary
stair shift. Enlarging the table while keeping old ids is also within
about 1.5%. The id move itself adds no material meshing cost. The broad
terrain-water row is within noise, unlike step 8.1's layout outlier.

The snapshot shifts reverse between runs: shared-light copy-on-write is
+14.1% in the broad set but -6.5% in the targeted set (483.1 / 451.5 ns),
and Arc cloning reverses from -5.7% to +13.6%. These are allocation and
layout-sensitive sub-microsecond rows, with unchanged code. Cloning seven
Chunks moves on the median but not the minimum. The shared-Chunk copy is
about 20 ns faster in the targeted run; it remains an allocation/layout
observation, not an algorithm change.

The full-agenda eviction row reproduces at 152.67 / 134.62 us (-11.8%) in
the targeted set. It has unchanged code and entirely id-independent input:
the same fixed Cell keys, HashMap retention and BTreeSet retention. The
inference is binary/container layout and hash-state sensitivity; no fluid
simulation rule or budget changed. All other fluid rows are within 3%.
The slab-fall shift is about 1.6 ns and reproduces at 41.6 / 40.3 ns; the
collision code and Shape geometry are unchanged.

Uniform/declined rows move by fractions of a nanosecond to a few
nanoseconds. Named ground adds an Option check and runtime Material ids to
the generator's uniform return, consistent with the 1.1 ns cave-band
uniform and 2.8 ns coarse-rock differences. The 3.2 ns LOD uniform return,
4 ns light early-outs and 13 ns declined-mesh return otherwise have
unchanged algorithms. Their percentages are not a simulation budget
change. The coarse-light surface row is at the edge of the 3% threshold.

The changed-LOD probe compares both ids in one executable and asserts
identical Chunk output and fluid overlays after normalizing ids. Three
rounds give dry old/moved medians 731.36 / 726.08 us (-0.7%), and flooded
1.230 / 1.246 ms (+1.3%). The longer cross-binary dry run spans 703.68 to
729.35 us on main and 712.34 to 755.05 us on the candidate; its ranges
overlap. Flooded falls to +1.5% on the targeted median. With unchanged
LOD algorithms and the same-binary check, these are layout/noise effects,
not a changed coarse derivation or a reason to alter simulation bounds.

The approved step is complete. No push or PR creation was authorized or
performed. A template-based copyable PR body is prepared in ../pr-body.md;
publishing it and pushing this feature branch remain separate actions.

### 2026-10-26: hover and scrolling follow-up for PR #21

The user requested a visible hover state and quick slots fixed at the
bottom centre, with the item selection scrolling independently. Tiles use
a brighter hover background and an accent border, retaining the selected
slot's white border. The selection uses Bevy's ScrollArea and a vertical
scrollbar; slots live in a separate footer. Assigning slots preserves the
browse position. Open-menu wheel input no longer selects a hotbar slot or
clears an outside-held item.

Added pointer-enter/exit styling coverage, native line/pixel scroll input
with a stationary footer and retained browse position, and a Client input
test for wheel events while the menu is open. Core code, Game/WIT content
and simulation bounds are unchanged, so the previous core A/B still applies.
Formatting and full CI pass: 878 tests passed, five existing measurement
tests remain ignored. The user reviewed and accepted the hover, scrolled
and full-size menu captures on 2026-10-26; the /tmp review folder was then
deleted.

### 2026-10-26: textured previews and drag feedback

Tickets 14 and 15 extend the reviewed picker with a model that follows a
slot drag and distinct Game-supplied preview textures. Final formatting,
review and ./scripts/cargo-ci pass: 885 tests, zero failures, five existing
manual measurements ignored. Guest/demo rebuild, Clippy, rustdoc, default
build, benchmark compilation, boundary checks and cargo-deny pass. The
Client still reads content and Server answers only; core algorithms and
benchmark inputs did not change. The prior interleaved measurements stand.

Native captures now drive the UI's actual drag/drop observers. Review
fixed missing visibility state on the atlas root and ineffective camera
dithering; those rendering warnings are gone. Bevy/winit still reports a
skipped Destroyed event while the capture app exits, after saving the PNG.
Physical OS pointer input is not automated by this capture driver.

The user accepted the drag feedback and the other textures, requesting
smoother water, clearer glass and flowing lava. Those three were revised;
review remains tracked in ticket 15. The current /tmp folder is retained
until that review. No new validation exceptions besides those above.

### 2026-10-27: shared World texture validation

Ticket 16 extends appearance to the World with one Game-authored texture
shared by terrain, previews and the 40-percent-opacity ghost. The final
combined tree passes formatting and ./scripts/cargo-ci: 886 tests, zero
failures and five existing ignored manual measurement tests. Full guest
rebuild, Clippy, rustdoc, default build, bench compilation, boundaries and
cargo-deny pass. Native Metal captures exercise the shared shaders and
transparent UI blending. Core and benchmark inputs did not change; the
prior interleaved evidence remains applicable. User visual review and the
retained folder are tracked in tickets 15 and 16. Ticket 16 records the
remaining capture/measurement limitations.
