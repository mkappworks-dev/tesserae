Status: ready-for-agent

# Generator determinism across platforms

A World regenerates every unchanged Chunk from its seed and pinned Generator
version, so a generator that answers differently on another platform or
toolchain shifts its terrain. This makes generation bit-identical on every
release target and keeps it so. It is also the precondition for revisiting
ADR-0031 (a saved Chunk is stored whole, not as a delta from generation). The
rule it sets is
[ADR-0032](../../docs/adr/0032-generation-uses-no-platform-maths.md).

## Problem Statement

A World made on one machine and opened on another can show different terrain
where nothing was changed. Rust's `std` transcendental functions, and `powi`,
call the platform's maths library or a compiler routine, and are documented as
varying by platform and Rust version. Versions 2 and 3 still use them:

- `sin`, `cos` and `atan2` in Settlement founding (where a Settlement's centre
  lands near the origin), Settlement roads and building orientation, and crop
  fields near water, from version 2;
- the same in version 3's level-disc test and its search for dry land that
  places a new Player;
- `powi` in Surface feature chance and tree crowding from version 2, and in
  version 3's landforms and wetness.

One flipped angle reroutes a road or moves a Settlement; a moved spawn drops a
Player somewhere else. Nothing would notice:

- The golden digests hash runtime Material and Shape ids, so they move with
  registration order and cannot be compared across content changes.
- They cover terrain only: no Settlement, Structure, Plant or Surface feature,
  no Overlay and no spawn. Today's version 2 digests run without ground, so they
  equal version 1's and exercise none of version 2.
- CI runs every job on Linux; releases build macOS arm64 and x86-64, Linux and
  Windows.
- `libm` is pinned only by the lockfile, so `cargo update` can change its
  answers.

Step 17 already moved version 3's landform `powf` and `exp` to `libm` and
recorded version 3 digests.

## Solution

- **The promise.** The same seed, Generator version, Voxel size and registered
  content give bit-identical generated Chunks, generated Overlay, Settlements,
  Structures and spawn on macOS arm64 and x86-64, Linux and Windows, and on
  every Rust toolchain from the workspace's `rust-version` on. Display-only
  output (sky and fog colour, audio, animation) is not covered.
- **Corrected in place.** Every non-deterministic float function that feeds
  generation in versions 2 and 3 moves to `libm` or to plain multiplication. No
  release has shipped, so every version 2 and 3 World is a development World;
  this is a one-time exception to the freeze, recorded in ADR-0032. After the
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
6. As an Engine developer, I want the golden digests to hash Material and Shape
   names, so that registering content in another order does not re-record them.
7. As an Engine developer, I want digests for Settlements, Structures, Plants,
   Surface features, the generated Overlay and spawn, so that drift in any of
   them fails a test rather than a Player's World.
8. As an Engine developer, I want each digest box to assert the content it is
   meant to cover, so that a digest cannot pass while covering empty ground.
9. As an Engine developer, I want every corrected call site reached by some
   digest case, so that the correction is measured rather than assumed.
10. As an Engine developer, I want the digests run on macOS, Linux and Windows
    on every PR, so that platform drift is caught at merge, not in a release.
11. As an Engine developer, I want the correction in its own commit after the
    name-based digests are recorded, so that its digest diff shows exactly which
    cases it moved.
12. As an Engine developer adding a landform, I want clippy to refuse `f32::sin`
    in generation code, so that I find out on Linux rather than from a
    Windows-only digest failure.
13. As an Engine developer working on audio, entity motion, sky colour or the
    Client, I want `std` maths to stay allowed there, so that the lint costs
    nothing outside generation.
14. As an Engine developer, I want the Server to ask the Generator for the spawn
    point, so that the rule a test digests is the rule the Server runs.
15. As an Engine developer revisiting ADR-0031, I want generation shown
    deterministic by name-based digests passing on all platforms, so that
    storing Chunks as deltas becomes a choice rather than a risk.
16. As a Game author changing Terra's content, I want the Engine's digests not
    to depend on Terra, so that tuning a Biome never re-records them.

## Implementation Decisions

- **What generation means here:** everything a Generator version derives from
  the seed: the generated Chunk, its generated Overlay (which the Plant table
  derives from the generated Chunk and the World's water), Settlement and
  Structure placement, and the spawn point. Climate's sky and fog blend is
  display and stays on `std`.
- **The correction.** `sin`, `cos` and `atan2` move to `libm` in Settlement
  founding and spawn, roads, building orientation, the level-disc test, crop
  field reach near water, and the dry land search. Every `powi(2)` becomes a
  multiplication, which should leave the digests unchanged. Version 1 has none
  of these and must not move. Basic float operations and `sqrt` are correctly
  rounded on every target and Rust never fuses them, so they stay.
- **`libm` is pinned to its exact version.** A `libm` update is a change to
  every Generator version and goes through the digest job like any other.
- **A spawn query on the Generator.** The Server's search (the nearest
  Settlement in widening rings, then the nearest dry land to the origin) moves
  into the Generator as one public query the Server calls. It is a real seam,
  not test API: the test digests the rule the Server runs.
- **Version 3 stays frozen afterwards.** The correction re-records the version 2
  and 3 digests once, in one commit whose message names the cases that moved.
  Known consequence, not a requirement: development Worlds get no migration and
  may show seams at the edge of saved Chunks and a moved spawn.
- **Digest content by name.** Each Voxel is hashed as its Material name, Shape
  name and Orientation; each Voxel's Overlay as its fluid's name, level and
  source flag (generation never sets fire), in one canonical encoding written
  once in the test.
- **Fixtures, not Terra.** The digests use the shared test fixtures.
  `tesserae-core` cannot load a Mod, and Terra's tuning must not re-record
  Engine digests. The fixtures gain a Biome that declares a Surface feature, a
  Column Plant that stands in water, a crop field that reaches toward water, a
  Structure Template and a Settlement with a near-spawn entry, so that every
  corrected call site is reached.
- **Cases.** Version 1 and the no-Biome version 2 case keep their origin box.
  Versions 2 and 3 also run with ground and Biomes, recorded separately; each
  `(version, seed, Voxel size)` case has the origin box, the high-ground box
  (version 3), a box on the spawn point sized to reach the Settlement's disc,
  and a box on a Structure. One version 3 case's origin stands in the sea, so
  the dry land search runs. The spawn point is found with the new query; the
  Structure by sweeping generated Chunks for a fixture Template's Material,
  rather than adding API for a test. Each box asserts what it holds (a
  Settlement building, a Structure, a Plant, water) before its digest is
  compared.
- **CI.** A `Determinism` job runs only the digest tests on `ubuntu-latest`,
  `windows-latest` and arm64 `macos-latest`, plus x86-64 macOS built for
  `x86_64-apple-darwin` and run under Rosetta on the arm64 runner. If Rosetta is
  unavailable there, the matrix drops x86-64 macOS with a note in the job; no
  self-hosted runner. The toolchain stays unpinned `stable`; the job is the
  canary for a toolchain update.
- **Lint.** The workspace's clippy configuration disallows the `std` `f32` and
  `f64` methods documented as non-deterministic: `sin`, `cos`, `tan`, `asin`,
  `acos`, `atan`, `atan2`, `sin_cos`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`,
  `atanh`, `exp`, `exp2`, `exp_m1`, `ln`, `ln_1p`, `log`, `log2`, `log10`,
  `powf`, `powi`, `hypot` and `cbrt`, with a reason pointing at ADR-0032. Every
  crate but `tesserae-core` allows them at crate level; inside `tesserae-core`,
  audio, entity motion, Plant growth, the sky blend, test modules, integration
  tests and benches allow them explicitly.
- **Order.** The lint lands before the correction, with a temporary allow on
  each generation site the correction removes; the correction deletes those
  allows, so the Lint job proves nothing is left. The Determinism job lands
  before the correction too, failing targets continuing on error and the job not
  yet required, so it records which platforms differ under `std` maths.
- **Names need a source.** The registered tables carry no names, so the test
  fixtures name each Material and Shape they register, and their new content is
  appended so no id other tests assert moves.

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
- Prior art: the version 1 to 3 digest tests and the high-ground mountain digest
  in the generator version test.
- The lint ticket shows `disallowed-methods` firing on method-call syntax
  (`x.sin()`) in a scratch build before relying on it.

## Out of Scope

- Storing saved Chunks as deltas from generation; ADR-0031 is revisited
  separately once this lands and storage is a measured problem.
- Determinism of the running simulation (fluid flow, Plant growth, entity
  motion) and of Mods' WASM code.
- Pinning the Rust toolchain.
- Migrating development Worlds made before the correction.
- Terra's own content digests.

## Further Notes

- The stub of this spec was written on the save incremental commits branch;
  whichever branch merges second takes this version.
- ADR-0031's "Revisit when" asks for exactly what this delivers. Its list of
  `std` calls still names version 3's terrain `powf`, which step 17 already
  moved to `libm`; correct that line when the branches meet.
