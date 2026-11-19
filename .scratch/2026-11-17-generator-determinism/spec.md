Status: done

# Generator determinism across platforms

A World regenerates every unchanged Chunk from its seed and pinned Generator
version, so a generator that answers differently on another platform or
toolchain shifts its terrain. This deletes Generator versions 1 and 2, leaving
version 3 as the only one, and makes it bit-identical on every release target
and keeps it so. It is also the precondition for revisiting
[ADR-0031](../../docs/adr/0031-a-saved-chunk-is-stored-whole-not-as-a-delta-from-generation.md).
The rules it sets are
[ADR-0032](../../docs/adr/0032-generation-uses-no-platform-maths.md) (no
platform maths) and
[ADR-0033](../../docs/adr/0033-generator-version-3-is-the-only-version.md)
(version 3 is the only version).

## Problem Statement

A World made on one machine and opened on another can show different terrain
where nothing was changed. Rust's `std` transcendental functions, and `powi`,
call the platform's maths library or a compiler routine, and are documented as
varying by platform and Rust version. Version 3 still uses them:

- `sin`, `cos` and `atan2` in Settlement founding (where a Settlement's centre
  lands near the origin), Settlement roads and building orientation, the
  level-disc test, crop fields near water, and the search for dry land that
  places a new Player;
- `powi` in Surface feature chance, tree crowding, landforms and wetness.

One flipped angle reroutes a road or moves a Settlement; a moved spawn drops a
Player somewhere else. Nothing would notice:

- The golden digests hash runtime Material and Shape ids, so they move with
  registration order and cannot be compared across content changes.
- They cover terrain only: no Settlement, Structure, Plant or Surface feature,
  no Overlay and no spawn.
- CI runs every job on Linux; releases build macOS arm64 and x86-64, Linux and
  Windows.
- `libm` is pinned only by the lockfile, so `cargo update` can change its
  answers.

And versions 1 and 2 are still in the code, behind version gates across
generation and the save, with most tests built on version 2's gentle land.
Making three versions deterministic, and carrying two nobody will ship, is work
with no Player behind it: nothing has been released, so every version 1 and 2
World is a development World.

Step 17 already moved version 3's landform `powf` and `exp` to `libm` and
recorded version 3 digests.

## Solution

- **Version 3 only.** Versions 1 and 2 are deleted. A World stamped 1 or 2
  refuses to open with a message naming its version; the number 3 stays, so the
  next revision is 4.
- **The promise.** The same seed, Generator version, Voxel size and registered
  content give bit-identical generated Chunks, generated Overlay, Settlements,
  Structures and spawn on macOS arm64 and x86-64, Linux and Windows, and on
  every Rust toolchain from the workspace's `rust-version` on. Display-only
  output (sky and fog colour, audio, animation) is not covered.
- **Corrected in place.** Every non-deterministic float function that feeds
  version 3's generation moves to `libm` or to plain multiplication. This is a
  one-time exception to version 3's freeze, recorded in ADR-0032. After the
  first release a fix to a Generator version is a new version.
- **Digests by name, covering every content kind,** recorded against today's
  code first so the correction's effect shows as a diff.
- **A Determinism CI job** runs the digests on all four targets on every PR.
- **A lint** refuses the non-deterministic float methods in generation code, so
  the next landform cannot bring one back.

## User Stories

1. As a Player, I want a World I copy from my Mac to my Windows PC to show the
   same terrain beyond what I built, so that no seam appears at the edge of my
   saved Chunks.
2. As a Player, I want a Settlement's roads, houses and fields to stand where
   they stood when I last played, on any machine, so that a village I know is
   still there.
3. As a Player joining a friend's World for the first time on another platform,
   I want to spawn where they did, so that the shared start is shared.
4. As a Player, I want a World to look the same after I update the game, so that
   a toolchain or dependency update never moves my terrain.
5. As a Player sharing a seed, I want the same seed and settings to make the
   same World on every platform, so that a seed is worth sharing.
6. As a Player opening a World from before version 3, I want a message saying
   its Generator version is no longer supported, so that I know why it will not
   open rather than seeing it broken.
7. As an Engine developer, I want one Generator version in the code, so that
   generation has no version gates and every test exercises the terrain Players
   get.
8. As an Engine developer, I want the golden digests to hash Material and Shape
   names, so that registering content in another order does not re-record them.
9. As an Engine developer, I want digests for Settlements, Structures, Plants,
   Surface features, the generated Overlay and spawn, so that drift in any of
   them fails a test rather than a Player's World.
10. As an Engine developer, I want each digest box to assert the content it is
    meant to cover, so that a digest cannot pass while covering empty ground.
11. As an Engine developer, I want every corrected call site reached by some
    digest case, so that the correction is measured rather than assumed.
12. As an Engine developer, I want the digests run on macOS, Linux and Windows
    on every PR, so that platform drift is caught at merge, not in a release.
13. As an Engine developer, I want the correction in its own commit after the
    name-based digests are recorded, so that its digest diff shows exactly which
    cases it moved.
14. As an Engine developer adding a landform, I want clippy to refuse `f32::sin`
    in generation code, so that I find out on Linux rather than from a
    Windows-only digest failure.
15. As an Engine developer working on audio, entity motion, sky colour or the
    Client, I want `std` maths to stay allowed there, so that the lint costs
    nothing outside generation.
16. As an Engine developer, I want the Server to ask the Generator for the spawn
    point, so that the rule a test digests is the rule the Server runs.
17. As an Engine developer revisiting ADR-0031, I want generation shown
    deterministic by name-based digests passing on all platforms, so that
    storing Chunks as deltas becomes a choice rather than a risk.
18. As a Game author changing Terra's content, I want the Engine's digests not
    to depend on Terra, so that tuning a Biome never re-records them.

## Implementation Decisions

- **Deleting versions 1 and 2 is a wide refactor,** done expand–contract. The
  shared test fixtures gain version 3 parameters beside today's version 2 ones;
  tests and benches move onto them in batches (core unit tests and benches, core
  integration tests, the Server, Client and Mod host); then one change deletes
  the version gates, versions 1 and 2's generation, their digests and the
  version 2 fixture parameters, and narrows the save's supported generator
  versions to 3. A test that depended on version 2's land is rewritten against
  version 3 or a hand-built Chunk, never deleted merely to pass. Benchmark
  figures move with it; a bench's earlier figures are not comparable across the
  switch.
- **Older Worlds** are refused through the save's existing refusal of an
  unsupported generator version, with a message that names the version and says
  it predates version 3. No migration.
- **What generation means here:** everything version 3 derives from the seed:
  the generated Chunk, its generated Overlay (which the Plant table derives from
  the generated Chunk and the World's water), Settlement and Structure
  placement, and the spawn point. Climate's sky and fog blend is display and
  stays on `std`.
- **The correction.** `sin`, `cos` and `atan2` move to `libm` in Settlement
  founding and spawn, roads, building orientation, the level-disc test, crop
  field reach near water, and the dry land search. Every `powi(2)` becomes a
  multiplication, which should leave the digests unchanged. Basic float
  operations and `sqrt` are correctly rounded on every target and Rust never
  fuses them, so they stay.
- **`libm` is pinned to its exact version.** A `libm` update is a change to the
  Generator and goes through the digest job like any other.
- **A spawn query on the Generator.** The Server's search (the nearest
  Settlement in widening rings, then the nearest dry land to the origin, then
  the origin) moves into the Generator as one public query the Server calls. It
  is a real seam, not test API: the test digests the rule the Server runs.
- **Version 3 stays frozen afterwards.** The correction re-records the version 3
  digests once, in one commit whose message names the cases that moved. Known
  consequence, not a requirement: development Worlds get no migration and may
  show seams at the edge of saved Chunks and a moved spawn.
- **Digest content by name.** Each Voxel is hashed as its Material name, Shape
  name and Orientation; each Voxel's Overlay as its fluid's name, level and
  source flag (generation never sets fire), in one canonical encoding written
  once in the test. The registered tables carry no names, so the test fixtures
  name each Material and Shape they register.
- **Fixtures, not Terra.** The digests use the shared test fixtures.
  `tesserae-core` cannot load a Mod, and Terra's tuning must not re-record
  Engine digests. The fixtures gain a Biome that declares a Surface feature, a
  Column Plant that stands in water, a crop field that reaches toward water, a
  Structure Template and a Settlement with a near-spawn entry, appended so no id
  other tests assert moves.
- **Cases.** Each `(seed, Voxel size)` case has the origin box, the high-ground
  box, a box on the spawn point sized to reach the Settlement's disc, and a box
  on a Structure. One case's origin stands in the sea, so the dry land search
  runs. The spawn point is found with the new query; the Structure by sweeping
  generated Chunks for a fixture Template's Material, rather than adding API for
  a test. Each box asserts what it holds (a Settlement building, a Structure, a
  Plant, water) before its digest is compared.
- **CI.** A `Determinism` job runs only the digest tests, in a release build, on
  `ubuntu-latest`, `windows-latest` and arm64 `macos-latest`, plus x86-64 macOS
  built for `x86_64-apple-darwin` and run under Rosetta on the arm64 runner. If
  Rosetta is unavailable there, the matrix drops x86-64 macOS with a note in the
  job; no self-hosted runner. The toolchain stays unpinned `stable`; the job is
  the canary for a toolchain update.
- **Lint.** The workspace's clippy configuration disallows the `std` `f32` and
  `f64` methods documented as non-deterministic: `sin`, `cos`, `tan`, `asin`,
  `acos`, `atan`, `atan2`, `sin_cos`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`,
  `atanh`, `exp`, `exp2`, `exp_m1`, `ln`, `ln_1p`, `log`, `log2`, `log10`,
  `powf`, `powi`, `hypot` and `cbrt`, with a reason pointing at ADR-0032. Every
  crate but `tesserae-core` allows them at crate level; inside `tesserae-core`,
  audio, entity motion, Plant growth, the sky blend, test modules, integration
  tests and benches allow them explicitly.
- **Order.** Versions 1 and 2 go first, so nothing after them digests, lints or
  corrects code about to be deleted. The lint lands before the correction, with
  a temporary allow on each generation site the correction removes; the
  correction deletes those allows, so the Lint job proves nothing is left. The
  Determinism job lands before the correction too, failing targets continuing on
  error and the job not yet required, so it records which platforms differ under
  `std` maths.
- **Version 3 defects found on the way** (a Surface feature lost at a Chunk's
  top, crop fields near water missing at 0.5 m, smoothing steps at one seed) are
  fixed inside the same pre-release exception, before the correction, so every
  move of version 3's output lands before the freeze resumes. A spawn that
  overlaps what is generated under the Player is fixed before versions 1 and 2
  go, since a test of it still needs version 2.
- **The Games are retuned for version 3** (Biome shares, tree spacing, bare
  Biomes, Settlement sizes, crop weights) so their tests pass under it with
  unchanged thresholds, and tests that pinned a place on version 2's map find it
  by search or on a hand-built World. Both land before versions 1 and 2 go.

## Testing Decisions

- The one seam is the existing golden digest integration test of
  `tesserae-core`, through public API: build a `Worldgen`, generate Chunks,
  derive their Overlay through the Plant table, and ask the new spawn query. The
  spawn query is the only new public API.
- A good test here compares recorded digests and asserts content presence; it
  never inspects which maths function produced a value.
- Recording: name-based digests are recorded on macOS against today's code;
  after the correction they are re-recorded once; from then on the four-target
  matrix must pass unchanged.
- Spawn: a recorded spawn position per case, compared bit for bit, alongside the
  digests.
- Refusal: a save test shows a World stamped 1 or 2 is refused with a message
  naming its version.
- Prior art: the version 3 digest tests and the high-ground mountain digest in
  the generator version test; the save's tests of unsupported versions.
- The lint ticket shows `disallowed-methods` firing on method-call syntax
  (`x.sin()`) in a scratch build before relying on it.

## Out of Scope

- Storing saved Chunks as deltas from generation; ADR-0031 is revisited
  separately once this lands and storage is a measured problem.
- Determinism of the running simulation (fluid flow, Plant growth, entity
  motion) and of Mods' WASM code.
- Pinning the Rust toolchain.
- Migrating or converting development Worlds of any version.
- Renumbering version 3.
- Terra's own content digests.

## Further Notes

- The stub of this spec merged with the save incremental commits branch; this
  version replaces it.
- The reference docs that describe versions 1 and 2 (the glossary, world
  persistence, data mods, contract compatibility, the design document's step
  notes) change in the ticket that deletes them, not before, so they never
  describe code that is not there.
