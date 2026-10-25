# 10: Demonstrate and document the completed simulation

Status: done
Blocked by: 09, 11

Spec: [acceptance and Further Notes](../spec.md).

## What to build

Ship an executable independent demo of the full step 8 behavior and document
the native state, Mod contract, units, approximation, and later-save boundary.
Leave integration with phase 7 and any future PR explicit.

## Acceptance criteria

- [x] The MIT demo declares lava, another fluid, combustible fuel, residue,
  and reactions through the published 0.2 contract without reaching core.
- [x] Reproducible controls/scenarios demonstrate waterfall/recession,
  waterlogging, cross-Chunk resume, contact reactions, fire behind/open around
  barriers, burning, extinguishing, and source removal.
- [x] Capture and inspect visible results and continuous mesh updates at
  supported Voxel sizes. Basic geometry/palette/flame indication is sufficient;
  water shading and Look-pack polish remain step 9.
- [x] Update the glossary, focused ADRs, architecture/build-order notes,
  and Mod documentation to describe accepted behavior and compatibility.
  Do not silently contradict existing authority, identity, unit, Job, or LOD
  invariants.
- [x] Document durable versus derived state for later save/load and current
  discard-on-eviction semantics, including transient exposure/fuel timing.
- [x] Document shared per-Voxel light, whole-Voxel wet-fuel protection,
  one-fluid-Material-per-Voxel, coarse LOD limits, and the measured budgets.
- [x] Retain all existing 0.1 guest tests and independent demo-boundary
  checks; verify Server-only mutation and native expensive algorithms.
- [x] Full scripts/cargo-ci and applicable live/native acceptance checks pass,
  with exact evidence and limitations recorded. All implementation tickets'
  completion state and Comments match their finishing commits.
- [x] Phase 7 PR status is not guessed. Record the need to inspect its actual
  eventual merge/ancestry before a phase 8 PR, preserving local dependency work
  and accounting for a possible squash merge. Do not push or merge under the
  planning authorization.

## Verification and completion

Run cargo fmt --all, npm run fmt, and full scripts/cargo-ci before committing.
Hand-wrap this file; never run Prettier on .scratch. In the finishing commit,
check the criteria, set Status: done, and append implementation, verification,
and evidence under Comments. Use Conventional Commits with:

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>

## Comments

- A future PR is separate authorization. This ticket does not merge phase 7,
  modify another checkout, or authorize a push from the planning session.
- 2026-10-25: done. The commit that closes this ticket is the one this
  comment lands in. Its trailer is `Co-Authored-By: Claude Opus 5.5`, not the
  Sonnet line above; the user accepted that.

### Implementation

- Fuel beside unheld terrain now keeps its progress. Before, a fuel Voxel
  with a face neighbour nobody held dropped its fire timer and, once the
  neighbour was held, started its whole ignition delay or burn again: a
  fire 7 s into an 8 s burn at the edge of the held World burnt 8 s more.
  The user chose, after a comparison with Luanti (persistent node timers that
  catch up on reactivation, ABMs that pause) and Minecraft (fire age kept in
  saved Chunks, no updates far from a player), the behaviour that looks
  right: the timer freezes. `fire::FireTimer` holds a `Clock`, `Runs { ready
  }` or `Frozen { left, solid }`; a paused evaluation freezes a timer that
  still describes the Voxel (an ignition of unburning fuel, a burn-out of
  burning fuel) and drops any other; the next evaluation that can decide goes
  on from `left`, unless the solid is no longer the one it froze for. Flow
  beside unheld terrain is unchanged: it still drops the change it waited
  for. Nothing lands from absent state: a frozen timer resumes only when a
  full evaluation wants the same change. Fuel in an evicted Chunk still
  loses everything with its Chunk. `fire.rs` module docs and the WIT 0.2
  `ignition-delay-s` doc say so.
- Demo (MIT, 0.2): a second fluid, `demo:mud` (12): 2 m reach, 1 m/s
  sideways, 2 m/s down, 1 m/s recession, 0.5 m reset, waterlogs,
  extinguishes, no heat; translucent (dimming 6, blended) and `demo:brick`
  (13). A second reaction, mud touching lava fires the mud to brick and
  keeps the lava, so one rule consumes the first side and one the second.
  Lava, wood, ash and lava/water to stone are as tickets 05 to 07 left them.
- Demo scenarios, built through Commands in metres and timed by the Mod from
  `tick` and `dt-s`, so they run the same at 1.0, 0.5 and 0.25 m. The build
  (94 Voxels at 1 m, 684 at 0.5 m, 5326 at 0.25 m, at the unchanged two a
  tick) finishes before any fluid moves; the Mod counts its reports to know.
  - the phase 7 wall and lantern, with its lava source, whose flow crosses
    the x = 0 Chunk face at every size;
  - a 14 m beam at 28 m: mud at its west end, lava 3 m in, water at its east
    end; mud meets lava and leaves brick, water meets lava and leaves stone,
    and the water runs through a row of fences, top slabs and posts on its
    way (waterlogging) before falling off both sides;
  - an 11 m pillar with a water source on top: a waterfall onto the ground
    that spreads 7 m, its source taken away 24 s in, and recedes;
  - a fire row on its own beam: wood lit at its top layer, a metre of open
    air to a wood cube and a wood stair that catch from it (fire spreads and
    leaves ash, the stair ash in its Shape), and a 2 m stone wall 5 m wide
    between the lit wood and a third wood cube, which never catches;
  - a burning wood cube on a beam of its own, with water poured on it 4 s
    in: it goes out still wood, and its water cannot reach the row.
- Dev only: `TESSERAE_VOXEL_SIZE_M` (1.0, 0.5 or 0.25; anything else warns
  and keeps 1.0) opens the World at that Voxel size, which nothing could
  choose before; `WorldSettings::from_env`.
- Docs: CONTEXT.md gains a Simulation section (Overlay, Occupants, Fluid,
  Source, Flowing, Logical step, Lifetime, Agenda, Rescan, Parked,
  Admission, Void, Waterlogging, Contact, Reaction, Receiving, Blocked
  contact, Exposure, Ignition progress, Burning, Burn-out, Flame indicator,
  Wet fuel, Coarse, Child, Waterline, Emitter bound), Contract line and
  Adapter, and Declaration, Edit and Command entries widened. ADR-0012 now
  describes the 0.2 line and the 0.1 adapter. New ADR-0013 (fluid and fire in
  an Overlay beside the Voxel, one reading, one fluid per Voxel, per-Void
  levels, shared per-Voxel light, whole-Voxel wet fuel, the durable-versus-
  derived inventory, eviction) and ADR-0014 (the kernel interface, logical
  steps and the event index, landing and staleness, rule knobs versus
  division knobs, idle terrain, the on-tick solve). DESIGN.md step 8 is done,
  with the measured budgets and the dev-profile caveat, and an Open item
  lists the known limits. docs/architecture.md lists the two ADRs. ADR-0011
  already described level 1 and the 256 m step (ticket 08).

### Known limits, recorded in DESIGN.md and the ADRs

- A writer changing the plane of light a neighbour's solve reads every tick
  can hold that neighbour's mesh back, by up to four spoilt solves
  (`SPOILT_BEFORE_ON_TICK`, ticket 09); before ticket 09 it was for ever.
- Level 1 is a mip-like volume vote: a one-Voxel stream on a coarse row with
  its floor is lost, waterlogged water mostly loses, coarse fluid is lit as
  air and emits nothing.
- Levels 2 and up are generator-only, so from level 2's inner edge, 256 m at
  the defaults, held fluid and fire disappear and generated water returns.
- Nothing climbs inside Shape geometry, and a lip is not modelled.
- The player's fluid tool places water only.
- New here: a fluid that is opaque and emits nothing draws its partial
  surfaces, and Shapes it fills, unlit, since light is one value per Voxel
  and the fluid makes the Voxel opaque. Found in the first capture, where
  mud was opaque and drew black; lava emits, so it never showed. The demo's
  mud is translucent instead, and per-Void light stays out of scope.
- Fuel beside unheld terrain keeps its progress (above); flow beside it
  still starts its delay again.
- Measured budgets are at the dev profile; a release build is faster.

### Budgets, as measured (dev profile unless marked)

| bound | value | evidence |
| --- | ---: | --- |
| `LANDED_PER_STEP` | 256 | ticket 09 sweep, 64 / 128 / 256 |
| `EMITTERS_PER_STEP` | 4 | ticket 09: 8 gave a 13.75 ms lava step |
| Agenda capacity + reserve | 16384 + 4096 | peak 9399, no overflow |
| rescanned a step | 4096 | about 9 ms of worker CPU a Chunk (release) |
| evaluations a Job, Jobs in flight | 1024, 4 | one step a tick but the widest |
| `SPOILT_BEFORE_ON_TICK` | 4 | ticket 09 starvation test |
| `MOD_VOXELS_PER_TICK` | 2 | phase 7, unchanged |

- Worst main-thread tick, thread CPU, over every ticket 09 scenario at 1.0,
  0.5 and 0.25 m: 11.17 ms (lava draining at 0.25 m), wall maxima 3.3 to
  11.2 ms, none over 15.625 ms, at one-minute loads of 1.5 to 7.6; control
  0.40 ms.
- My brief said `EMITTERS_PER_STEP` 8 and a worst tick of 10.2-11.2 ms. The
  code and ticket 09 say 4; 8 was measured and rejected. 10.29 ms and
  11.17 ms are the 1 m and 0.25 m lava wall maxima at 4. The docs use the
  code's value.
- Ticket 11 (release, thread CPU, against 2ae7399): dry `greedy_mesh` and
  `mesh_chunk` 15-45% faster, `lod_mesh_coarse*` 32-45% faster, bulk light
  -12% to +10% (within the rounds' spread), Shape registration +8-15% (4.81
  ms for the built-in table), the Void derivation itself.

### Verification

- Test functions: core lib 320 (320; one replaced), meshing 68, Mods 89
  (three ignored), `tesserae` binary 348 (347; two ignored as before);
  workspace 825 (824). Full `scripts/cargo-ci` passed on the tree this
  commit records: rustfmt (workspace and demo), Prettier, clippy, default
  build, guests, tests, bench compile, demo boundary and Bevy/wgpu
  boundaries, cargo-deny.
- New or changed tests:
  - `flow::tests::fire::fuel_beside_unknown_terrain_waits_and_keeps_its_
    progress` (replaces `unknown_terrain_never_wets_fuel_and_fuel_beside_it_
    waits`), at 1.0, 0.5 and 0.25 m: fuel halfway through its delay waits
    out 3 delays beside unheld terrain with its timer kept and catches the
    rest of its delay (+-2 steps) once held; burning halfway, it neither burns
    out nor goes out over two burns unheld and burns out the rest once held,
    to ash; water unheld never wets and, held again, puts it out; a wood cube
    replaced by a wood stair while it waits gets a fresh delay.
  - `server::fluids::tests::fire::eviction_pauses_progress_and_readmission_
    without_the_fire_drops_it` (was `eviction_takes_exposure_...`): the wood
    keeps one frozen timer while the brazier's Chunk is gone and never
    catches; readmitted without the fire, the timer is dropped, and lit again
    it catches a fresh delay later.
  - `server::mods::runtime::tests::demo_contract_content_build_and_scenarios_
    reach_real_voxels_and_light` (was `..._first_batch_...`), at 1.0 and 0.5
    m with the real WASI component: mud and brick at 12 and 13 and their
    appearance, mud's row, both reactions resolved by name with the side
    each consumes, the build's 16 Commands and Voxel counts with nothing
    rejected, fixtures where they belong, the lamp at 15 and lava lighting
    the Voxel above it to 14, no scenario before the build has applied, then
    four sources and two lit fuels with nothing rejected, the douse source
    255-257 ticks later, the waterfall's source gone by 24 s, nothing more.
  - `world::tests::the_dev_voxel_size_is_one_of_the_tested_sizes_or_the_
    default`.
- Mutation checks, each failing the new kernel test: dropping the timer on a
  pause as before (fails at the frozen-timer count); resuming a frozen timer
  whatever the solid (fails at the stair replacement).
- Existing 0.1 fixtures and the demo boundary check pass unchanged (CI's
  guests and Boundaries steps). No Client or render code reaches
  `ChunkStore`, `set_occupants`, `FluidSim` or the Mod queues (grep), and no
  new Server mutator was added; `tesserae-core` gained no dependency and
  `Cargo.lock` is unchanged; Wasmtime/WASI 49.0.4 and Cranelift 0.136.4 and
  the two-Voxel drain cap are as they were.
- Ticket audit: tickets 01 to 09 and 11 have `Status: done`, every criterion
  ticked, and were last changed by their own finishing commits (d849e14,
  35b5f2f, 3c4dad0, 8262444, 9da5f25, b48781a, a47710c for 07's amendment
  after 8c43deb, 585ae2c, b454e28, 539dbca).

### Live captures

- Offscreen (`TESSERAE_SCREENSHOT_OFFSCREEN=1`), dev build, the demo from a
  scratch Mods directory with Trust granted by `TESSERAE_TRUST_MODS=1`, at
  `TESSERAE_VOXEL_SIZE_M` 1.0, 0.5 and 0.25, from the spawn Eye 18 m over
  the ground, aimed at the fire row (yaw 66, pitch -40) and at the beam and
  waterfall (yaw -64, pitch -42), at several times after the fill. Uptime
  before and after each run is in `/private/tmp/tesserae-p8-t10.a/
  captures.log` (one-minute load 3.6 to 25 at the run edges); the images
  are in `/tmp/tesserae-phase8-review/`, not committed, for review and then
  deletion.
- Seen at every size: the fire row's lit wood burning with flame quads, the
  next wood catching across the gap, both to dark-grey ash in their Shapes
  (the stair's ash a stair), the wood behind the wall intact; the doused
  wood out under its water and still wood; on the beam, water through the
  waterlogged fences (lattices of small fences at 0.25 m), slabs and posts,
  stone where lava met water and brick where mud met lava, lava glowing on
  the wall and the beam and spilling to the ground; the waterfall feeding a
  pool 7 m across and, after its source went, the pillar dry (at 0.25 m one
  frame caught the column draining from the top). Captures at successive
  times show the meshes following each change.
- Limits: guest log lines are kept in the host's buffer and not printed by a
  live run, so the scenario's start was read off the images, not a log; at
  0.25 m the 160 m view fills in about 65 s at the dev profile and the build
  takes about 42 s more at two Voxels a tick. Captures are not measurements:
  no figure above comes from them.

### Before any phase 8 PR

- Phase 7's merge state is not guessed. Here, `origin/main` (as last fetched)
  is ee77e54, phase 7's squash merge (#18), and this branch is 12 commits on
  it; the local `main` ref is stale at a95567d. Before a PR, fetch, check
  that ee77e54 or its successor is still `origin/main`'s ancestry, rebase if
  `main` moved, and keep local dependency work. Nothing was pushed or merged,
  and no other checkout was touched.
- The PR should carry ticket 09's hot-path comparison as superseded by
  ticket 11's, and the open items above.
