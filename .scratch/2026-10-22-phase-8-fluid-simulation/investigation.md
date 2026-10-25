Superseded by [spec.md](./spec.md); kept as the record of how phase 8 was planned.

# Step 8: fluid simulation investigation

Planning in progress. These notes distinguish agreed scope from open choices;
they are not implementation authorization or ready-for-agent tickets.

## Starting point

- Bound worktree: `feat/phase-8-fluid-simulation`, initially clean at
  `a60d1f998a921898df72eb4bc71c98f536937c2a`.
- Phase 7 is the local dependency. Its PR is
  <https://github.com/mkappworks-dev/tesserae/pull/18>; merge status is
  unverified. Do not merge it. Account for its eventual merge before opening
  the phase 8 PR. This session does not authorize a push or merge.
- Keep Wasmtime/WASI 49.0.4 and Cranelift 0.136.4 security fixes.
- spec.md and ten separate numbered issues exist in this directory. Reviewed
  on 2026-10-22; every issue is ready-for-agent.

## Agreed behavior

- Use permanent sources and dependent flowing fluid. Generated lake water
  supplies flow without draining as a finite conserved volume.
- Flow downward and spread sideways over a bounded distance in metres.
- Recede when supply is removed. Flow does not create new sources.
- Do not fill connected Caves upward to the lake's waterline through pressure
  or connected-volume equalization.
- Pause at unloaded Chunk boundaries, and wake when neighbors load. Fluid
  simulation does not load more terrain on its own.
- Follow existing discard-on-eviction behavior. Durable state is deferred
  until save/load is implemented; no offline catch-up policy is implied.
- Use deterministic simulation steps and bounded work. Under load the
  simulation slows rather than skipping updates or exceeding its budget.
  Equal loaded terrain and ordered Edits must produce equal results at equal
  simulation progress, independent of worker completion timing.
- Include waterlogging, lava, and Mod-defined fluids in step 8. The user
  explicitly chose all three, overriding the proposed water-only scope.
- Include fluid contact reactions, heat, fire, and burning in step 8. The
  user explicitly included all four; thermal effects are not deferred.
- Use local heat exposure, delayed ignition, fuel consumption, declared
  residue, and water suppression. Solid barriers block exposure; heat does
  not conduct through solid terrain. The user approved this recommendation
  after the Minecraft/Luanti comparison.
- Keep expensive algorithms native in tesserae-core. Mods declare data;
  they do not run fluid algorithms or per-Voxel callbacks in WASM.
- Water shading, Look packs, and visual polish remain step 9. Functional
  geometry and lighting needed to show simulated state must be specified
  separately from shading polish.

## Code findings

- A Voxel stores Material, Shape, and Orientation in a u32. There is no flow
  level, source flag, or second Material for waterlogging. Shape and
  Orientation bytes already have defined meanings; none is spare fluid state.
- Chunk storage is palette-compressed, with a zero-allocation uniform case.
  There is no Chunk serialization or application save/load implementation.
- Built-in water is Material 7: Blend rendering, same-Material occlusion,
  dimming 2, and no collision or Edit-ray stop. Non-solid does not mean that
  arbitrary Mod Materials may be overwritten by a fluid.
- World generation places full-cube water in open space above terrain below
  the waterline. It leaves carved Caves dry and avoids underwater smoothing.
- The existing hotbar already places full-cube water. Player placement can
  replace water with solid geometry because picking ignores non-solid water.
  Source removal and waterlogging need explicit Edit semantics.
- Tables::vet_placed is shared validation. The Mod region conversion and lazy
  BoxCursor already exist and should be reused where applicable.
- Server-private set_voxel validates and normalizes a Voxel, enforces the
  shaped cap, changes the Chunk, immediately relights, and invalidates meshes
  and the level-1 cover. Player history records player Edits only.
- Phase 7 measured a Mod drain cap of two Voxels per tick because each write
  pays for relight. Its full 32-cubed fill takes about 256 seconds at 64 Hz.
  These are prior measurements, not new fluid measurements or a fluid budget.
- Bulk streaming light solves converge upward from existing boundary light.
  They cannot simply replace the decrease/increase Edit relight path. A
  reaction, new water, lava emission, or removed fluid can darken the World.
- Jobs read Arc Snapshots and return owned results. Only the Server lands
  results. Existing timing is asynchronous, so fluids need a deterministic
  scheduling and commit policy beyond reusing worker completion order.
- Level 1 derives from held level-0 Chunks; higher levels use generation.
  Existing level-1 invalidation must also observe fluid state changes.
- Coarse water lighting uses the generated waterline. A coarse mesh correction
  also aligns nearby upward water faces to that line. Neither rule should be
  assumed valid for arbitrary flowing water, lava, or Mod-defined fluids.
- Shape registration derives geometry, face coverage, and volume from a
  rotated occupancy grid. Connections are derived from neighbors. Waterlogging
  needs explicit permeability and rendering rules for those geometries.
- The current WIT Declaration has no fluid definitions. Its Voxel and Command
  forms cannot independently describe solid and fluid occupants. Contract
  evolution and compatibility are therefore part of the expanded scope.
- Neither the existing Material table nor the WIT contract represents
  temperature, heat transfer, ignition, fuel, or burn residue. Thermal
  behavior needs native state and validated declarations as well as fluids.

## Open decisions

- Fluid contact reactions are proposed as declarative data executed natively.
  Reaction products, matching rules, and conflicting-contact priority remain
  to be specified.
- Heat and combustion: finalize exposure attenuation, geometry-aware passage,
  ignition/fuel timing, and extinguishing rules within the approved local
  model. Spatial rates and reach use metres; elapsed time uses seconds.
- Waterlogging: permitted Materials/Shapes, internal empty-space connectivity,
  connected arms, flow passage, clipping, and placement/removal behavior.
- Fluid data and WIT evolution: source/state representation, validated rates
  and ranges, reaction ownership, references, and compatibility strategy.
- Timing: numeric tuning, metre scaling, deterministic conflict resolution,
  Edits during an incomplete step, fairness, bounded retained work, and stale
  Snapshot behavior.
- Rendering and light: functional surface heights, fluid/solid composition,
  lava emission, removal correctness, LOD approximation, and mesh progress
  under sustained changes.
- Testing seams and explicit performance acceptance, including whether a
  bounded bulk relight path is necessary. It remains a named follow-up until
  evidence or an agreed acceptance requirement makes it part of this phase.

## Verification and ticket requirements

- All World mutation stays Server-private; core has no Bevy or wgpu edges.
  Spatial physical quantities use metres and the World's Voxel size.
- Measurements require uptime before and after every run, load beside each
  figure, interleaved CPU comparisons, discarded distorted loads, and unique
  temporary paths. No new measurements have been run for these notes.
- Exclude the known fbm2 debug overflow near u32::MAX seeds from this phase.
- Hand-wrap .scratch Markdown at 80 columns; never run Prettier on it.
- Before commits, run cargo fmt --all, npm run fmt, and full scripts/cargo-ci.
  Keep each finished ticket's done status and Comments in its finishing commit.
- Use Conventional Commits and the requested trailer:
  Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>.

## Comments

- The user confirmed that step 8 includes fluid contact reactions, heat,
  fire, and burning. The earlier proposal to defer thermal effects was not
  accepted.
- The user requested a recommendation grounded in Minecraft and Luanti.
  Proposed, not yet agreed: local heat exposure with ignition delay, finite
  burning and declared residue, water suppression, and solid barriers blocking
  exposure. This includes heat effects without conductive temperature state
  throughout solid terrain. A primary-source comparison is being captured
  separately in thermal-prior-art.md.
- The user subsequently approved that local-exposure recommendation. The
  primary-source comparison is complete in thermal-prior-art.md. The spec
  now records proposed engineering choices and acceptance for review.
- An independent builder check identified missing fuel, reach-renewal,
  lighting-resolution, and unmatched-fluid policies. The spec now proposes
  whole-Voxel wet-fuel protection, metre-valued reset drops, existing shared
  per-Voxel light, and unmatched fluids blocking each other. These calls are
  for user review, not prior approvals.
- Thirteen Markdown files, including ten issue drafts, passed checks for
  80-column wrapping, trailing whitespace, local links, status metadata,
  Comments headings, numbering, and backward-only dependency edges. No
  implementation or commit has been made; full cargo-ci has not been run
  for these draft-only changes.
