# Cut the CI Test job's test time

Off the build order, so the branch is `tech/ci-test-timing`. No tickets: the
change is confined to `.github/workflows/ci.yml`. It builds on
[ci-test-job-speedup](../2026-10-31-ci-test-job-speedup/spec.md).

## Why

After that change `Test` still took about 13 minutes (run 41227953134). One
"Run tests" step of 12m19s hid whether compile or execution dominated, and the
fix differs.

## What changes

1. "Run tests" is split into `Build tests` (`--no-run`) and `Run tests`, with
   identical flags so the second compiles nothing. The first measurement:
   Build tests 107s, Run tests 635s. Execution dominates, and clippy in `Lint`
   takes 11s, so compiling twice is not the cost.
2. Unit tests run under `cargo-nextest`, with `cargo test --doc` kept because
   nextest skips doctests. Run tests fell to 508s.
3. `tesserae-core` is compiled at `opt-level=3` in the test steps, by a
   `--config` flag so local builds keep `[profile.dev]`. Its property and
   brute-force meshing and LOD tests are CPU-bound; the slowest fell from 191s
   to 26s.
4. The stale "37 unit tests" comment is corrected.

| Step        | Before | Nextest | + `tesserae-core` at opt 3 |
| ----------- | ------ | ------- | -------------------------- |
| Build tests | 107s   | 106s    | 115s                       |
| Run tests   | 635s   | 508s    | 305s                       |
| Job total   | 814s   | 679s    | 494s                       |

## Not in this change

Optimising the `tesserae` crate or wasmtime in test builds. The first slows the
largest compile; the second undoes a deliberate `opt-level=0`. The slowest test
is now a 90s server test, so this is for a ticket if it matters.

## Done when

- `Test` is measurably below the 13 minutes it took before.
- `Test` and `Benches` both pass on the PR.
