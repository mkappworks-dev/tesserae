# games/demo, built in CI

Status: done
Blocked by: 07, 08

Spec decision 13 and ADR-0004's promise.

## What to build

- `games/demo/`: a `cdylib` for `wasm32-wasip2` with its own `Cargo.lock`,
  `wit-bindgen` pointed at `../../crates/tesserae-mods/wit`, `mod.toml` with
  `root = true`, an MIT `LICENSE` file, and nothing in its sources or manifest
  that reaches into `crates/` except that `wit` directory.
- Content, exercising each part of the contract once: a Material `demo:lamp` (an
  emitter) with an appearance colour, the fence as a connected Shape declared as
  data, a `world` (via `default-world-params`, edited) with lower relief and no
  water, a `view` of 160 m and 3 000 m, and on the first `tick` a `fill` and a
  `place` that build a short wall and a lantern post beside the spawn point, at
  positions in metres.
- `scripts/build-guests` also builds `games/demo` and copies `demo.wasm` and
  `mod.toml` to `games/demo/dist/` (gitignored).
- CI's Test job builds it; a test (in `tesserae`, so it can use the Server's
  edit path headlessly) loads `dist`, runs `declare` and one `tick`, applies the
  Commands to a held, lit `ChunkStore` through the drain, and asserts the
  Voxels, the registered ids and the parameters.
- A screenshot per `TESSERAE_SCREENSHOT_OFFSCREEN=1`, kept in one `/tmp` folder
  for the review and deleted after: the wall, the lantern post lit, the lamp
  Material in its colour, and a view past the default reach. Show it to the
  user; never commit it.

## Tests

- The load-and-apply test above.
- A grep test, or a short script run by `cargo-ci`, that `games/demo` mentions
  no path into `crates/` other than the `wit` directory.
- Run with the dev fence on: no second fence registered.

## Acceptance criteria

- `./scripts/cargo-ci` passes; CI's Test job builds and runs the demo.
- `cargo run -p tesserae -- --mods games/demo/dist --trust-all` opens a World
  with the demo's content and the screenshots exist.

## Comments

### Implementation and review (2026-10-22)

- Added the MIT demo as an independent wasm32-wasip2 cdylib with its own
  lockfile. Only wit-bindgen and the shared WIT directory cross its boundary.
  It declares an orange emitter and the connected fence, sets relief to 4 m,
  disables water and caves, and requests 160 m / 3,000 m view distances.
- The first tick returns one Fill and one Place in metres. Registered names
  resolve every Material and Shape id. The below-ground foundations avoid a
  terrain-query API and keep the small wall and lantern beside the spawn.
- First Mod dispatch waits once for the spawn's 3x3x3 Chunk neighborhood to be
  held and lit, avoiding startup Commands being skipped or overwritten by the
  bulk light solve. Subsequent unloaded targets still skip through the normal
  drain; this is no guarantee for arbitrary distant first-tick construction.
- The headless integration test now uses the real demo and startup gate, then
  drains all 38 Voxels at 1 m and all 255 at 0.5 m with zero rejections. It
  asserts registered ids, parameters, appearance, connected geometry, lamp
  block light 15, the one-time build, and no extra dev fence. The separate
  startup regression retains the original red/green evidence.
- Build-guests produces games/demo/dist/demo. CI and cargo-ci build it, check
  the WIT-only boundary and independent rustfmt, and run the dev-enabled suite.
  The boundary negative check rejects an injected direct engine import.
- Validation logs are under target/phase7-runs: t09-cargo-ci.log (first full
  green run), t09-demo-startup-final.log (strengthened integration test),
  t09-boundary-negative.log, t09-format.log, and t09-final-cargo-ci.log (final
  full run). The full suite has 613 passing tests and four explicitly ignored
  manual measurements. No benchmark was run or inferred from capture timing.
- The exact normal cargo run command loaded the demo, opened a Metal World,
  reached startup readiness, and exited 0 after a deliberate SIGINT; see
  t09-normal-startup.log. Offscreen capture logs are t09-live-spawn.log,
  t09-live-content.log and t09-live-distance.log. Visual inspection confirms
  the wall, lantern post, orange lamp and distant low-relief terrain.
- Review images remain in /tmp/tesserae-ticket09-review.iS5j1f: spawn.png is the
  initial clipped framing, content.png shows the complete wall and lantern,
  and distance.png shows the horizon. The parent must show these images and
  delete the folder only after review. Images are not committed.
- Wording limitation: the specified 3,000 m LOD reach is below the current
  4,096 m engine default. The distant image demonstrates terrain beyond the
  default 192 m full-detail radius, not beyond the default LOD reach. The
  specified demo values and engine defaults are retained.
- Corrected the WIT arm-face documentation to the existing -x/+x/-y/+y/-z/+z
  runtime order; no contract encoding or runtime face mapping changed.
