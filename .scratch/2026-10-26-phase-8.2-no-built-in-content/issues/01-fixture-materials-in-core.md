# 01: Fixture Materials, and the core suite on them

Status: done
Blocked by: None (can start immediately)

Spec: [decision 21](../spec.md).

## What to build

The expand step, and the first migration batch of the step's wide refactor.
`tesserae-core`'s test support gains register calls beside the fixture water:

- a ground call that registers stone, dirt and grass with today's rows and
  returns the ground's three ids, in the shape ticket 03's ground parameters
  will take;
- one call each for lantern, glass and leaves, returning the id each got.

The deterministic test terrain takes the ground's ids as an argument, so it no
longer names a built-in constant. Every test and bench in `tesserae-core` moves
to these calls and the ids they return. The built-in constants and rows still
exist, so production code and the other two crates are untouched and the suite
stays green. Benches keep their workloads: the same terrain, the same rows,
only the ids move to 8 and up.

Production code that still compares against a built-in id (World generation's
ground) keeps its tests on the built-in ids until ticket 03. List each such
site in the Comments with the ticket that moves it, as step 8.1's ticket 01
did.

## Acceptance criteria

- [ ] Each fixture registers like any Mod Material, at the next free id, with
      the row the built-in has today. No fixture has a fixed id, and nothing
      outside tests and benches calls them.
- [ ] A test pins each fixture's row against the numbers in the spec, not
      against the built-in row.
- [ ] No `tesserae-core` test or bench names a built-in solid constant, except
      the sites listed in the Comments with the ticket that moves each.
- [ ] Every assertion that depended on a built-in id asserts on the fixture's
      id, never on a literal.
- [ ] The meshing, light, LOD, collide and fluids benches register the
      fixtures and build the same Chunks as before.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

The register calls return ids from MaterialTable registration; the ground
returns GroundParams for ticket 03. Shared test support registers those rows
and water together, so every algorithm reads the table that owns its ids.
Terrain and checkerboard now accept their Materials. The terrain hash maps
registered ids back to layer labels to keep the original workload pinned.
The only app change adapts chunk_mesh's terrain test to the new signature.

Exceptions until ticket 03: worldgen's generation and smoothing tests;
light's generated cave-mouth surface checks; lod's generated surface cap
check; meshing's cave fixture scan. Generated terrain in benches likewise
keeps its built-in ground until the generator accepts ground parameters.
MaterialTable's built-in row/id tests remain until ticket 06. Standalone
ReactionTable tests use arbitrary ids without consulting MaterialTable.
The testgen solid re-exports remain for production callers until migration.

Formatting, review and full CI are deferred to the end of the phase at the
user's request on 2026-10-26. Acceptance checks will be recorded there.
