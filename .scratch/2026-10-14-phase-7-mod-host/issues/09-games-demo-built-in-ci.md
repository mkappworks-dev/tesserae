# games/demo, built in CI

Status: ready-for-agent
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
