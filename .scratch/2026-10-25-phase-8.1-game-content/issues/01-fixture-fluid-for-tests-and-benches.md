# 01: Tests and benches register their own fixture fluid

Status: ready-for-agent
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

- [ ] The fixture registers like any Mod Material, at the next free id, with
      the properties and fluid row water has today. Nothing outside tests and
      benches calls it.
- [ ] No test or bench in `tesserae-core`, `tesserae-mods` or `tesserae` refers
      to the built-in water Material constant or the built-in water fluid row.
      Production code still may until ticket 09.
- [ ] Assertions that depended on water's id being 7 assert on the fixture's
      id, never on a literal.
- [ ] Every test passes unchanged in meaning. The renderer's blend-order and
      LOD water-cut tests draw the fixture fluid.
- [ ] The meshing, LOD and fluids benches register the fixture fluid. An
      interleaved CPU-time A/B against main shows no change beyond noise.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
