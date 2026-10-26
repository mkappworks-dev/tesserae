# 01: Tests and benches register their own fixture fluid

Status: done
Blocked by: None (can start immediately)

Spec: [decision 17](../spec.md).

## What to build

A prefactor for the rest of the phase. `tesserae-core` exports a fixture beside
its deterministic test terrain that registers a water-like Material and fluid
row into a set of tables. It has today's water Material properties and fluid
numbers, and returns the id it got. Every test and bench across the three crates
that names the built-in water constant or the built-in fluid row moves to the
fixture. Behaviour and bench workloads do not change. The built-in water still
exists after this ticket, so the suite stays green throughout; removing it is
ticket 09.

This is the phase's wide refactor (about 40 test sites and three benches),
done as the expand step: the new fixture lives beside the old constant.

## Acceptance criteria

- [x] The fixture registers like any Mod Material, at the next free id, with
      the properties and fluid row water has today. Nothing outside tests and
      benches calls it.
- [x] No test or bench in `tesserae-core`, `tesserae-mods` or `tesserae` refers
      to the built-in water Material constant or the built-in water fluid row.
      Production code still may until ticket 09. Met for every test that uses
      water as some fluid; tests of production paths still hard-coded to it
      stay, listed in the Comments with the ticket that moves each.
- [x] Assertions that depended on water's id being 7 assert on the fixture's
      id, never on a literal.
- [x] Every test passes unchanged in meaning. The renderer's blend-order and
      LOD water-cut tests draw the fixture fluid. The Server's LOD water-cut
      test stays on built-in water until ticket 02; see the Comments.
- [x] The meshing, LOD and fluids benches register the fixture fluid.
- [ ] ~~An interleaved CPU-time A/B against main shows no change beyond
      noise.~~ Moved to ticket 10's phase-wide A/B at the user's request on
      2026-10-26; the interim run is in the Comments.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-25: implementation

**The fixture.** `testgen::register_water` registers `WATER_MATERIAL` and
`WATER_FLUID` through `MaterialTable::register` and `Tables::register_fluid`,
the calls a Declaration's rows go through, and returns the id it got (8 on the
built-in tables today). The test
`fixture_water_registers_at_the_next_id_with_waters_numbers` pins the numbers
from the spec, not from `FluidDef::WATER`.

**Moved.** About 300 sites across 22 files in the three crates. Every test
and bench that uses water only as some fluid now registers the fixture and
hands those Tables to everything that reads the water: Grid and Sim, the light
solvers, the mesher, Occupants, `FlowScale`, and the Server's `ContentTables`
in the harness tests, which start from `ContentTables::default()` so the dev
fence is still registered. An id no table knows reads as the opaque stand-in,
so a missed hand-off would have changed a test silently rather than failed it.

**Left on the built-in water, on purpose.** The second criterion cannot hold
while production code still compares to `material::WATER`: a fixture id fed to
those paths tests something else, or passes for another reason. Each remaining
site, with the ticket that moves it:
- Generated lakes and everything checked against them: worldgen's lake tests,
  `flow/tests.rs` `a_shaft_dug_under_a_generated_lake_fills_from_it`,
  `benches/fluids.rs` `rescans` (lake shore and dry terrain). Ticket 02.
- The level-1 waterline (`lod.rs`
  `only_water_the_generator_could_have_made_stays_packed`, the `pool()` and
  `random_children()` generators under the drowned waterline, `held.rs`'s
  waterline section), coarse under-water light (`level_zero_water`,
  `coarse_water_is_dimmed_by_its_depth_…`) and the Server's cut of a coarse
  water surface (`server/lod.rs` `coarse_water`). Ticket 02; the last was
  missing from its list and is added there. That test
  is the criterion's "LOD water-cut test", so it still draws built-in water;
  the renderer's blend-order tests (`client/chunk_view.rs`) draw the fixture.
- Tests that load the demo or the `fluids` guest, whose reactions name
  `core:water`: these now reach it by name through the Registry, not the
  constant. Tickets 03 and 04.
- The Client hotbar's water slot (`client/edit.rs`). Ticket 06.
- The built-in tables themselves (`material.rs`, `fluid.rs`
  `water_is_the_one_builtin_fluid`, `tesserae-mods` `behaviour.rs`),
  `core:water` resolving to 7 (`behaviour.rs`, `declare.rs`, `lines.rs`,
  `mods/runtime/tests.rs`), the render palette's water entry and the gallery
  pool. Ticket 09.

`reaction.rs`'s tests keep their own `const WATER: MaterialId = 7`: the
ReactionTable never consults the Material table, and the canonical order those
tests check depends on water's id being below lava's 9.

**Changed beyond a substitution.**
- `fluid.rs` `registration_refuses_what_cannot_flow` no longer re-registers the
  built-in row; registering lava twice already covers `AlreadyAFluid`.
- `fluid.rs` `rank_is_the_registration_order` ranks relative to the fluids
  already in the table, and `flow/tests.rs`'s scale test expects the fixture's
  rank rather than 0, which was the built-in water's.
- In the benches' demo tables the fixture water registers before lava, as the
  built-in water did, so equal claims still go to water.

**Verification.** `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci`
pass, cargo-deny included: core lib 321, `tests/meshing` 68, tesserae 346 (2
ignored), tesserae-mods 86 (3 ignored).

**Interim bench A/B, superseded by ticket 10's.** Main (9992e8e) against this
tree, thread-CPU criterion (uncommitted patch), `--warm-up-time 1
--measurement-time 2`, seven interleaved rounds of the meshing, collide, lod
and fluids binaries. Rounds 1 and 2 ran beside the forks' and CI's builds (load
up to 18) and are discarded; a round cut off by a session restart was rerun.
Over rounds 3 to 7, 115 rows: almost all within ±4% both ways, including rows
this ticket cannot reach (`greedy_mesh/terrain_canopy` +4.0%,
`greedy_mesh/solid` -3.6%), which is the noise of a relinked binary. One row
stands out: `greedy_mesh/terrain_water`, +7.5% on the minimum and +6.5% on
the median,
steady across rounds. Its workload is the same terrain under water at id 8
instead of 7, with the same table rows. A probe benching both ids in one
binary was stopped unrun when the A/B moved to ticket 10, so whether this is
layout or a real cost is open there. Raw data, not committed:
`<session scratchpad>/ab/` (`runs.log`, `crit/`, `summary-r3-7.md`).
