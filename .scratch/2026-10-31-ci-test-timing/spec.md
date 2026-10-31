# Find where the CI Test job spends its time

Off the build order, so the branch is `tech/ci-test-timing`. No tickets: the
change is confined to `.github/workflows/ci.yml`. It builds on
[ci-test-job-speedup](../2026-10-31-ci-test-job-speedup/spec.md).

## Why

After that change `Test` still takes about 13 minutes (run 41227953134). The
step timings from the API are: cache 25s, guests 27s, "Run tests" 12m19s. That
one step compiles the workspace test binaries and then runs 1,326 tests
(`tesserae` 744, `tesserae-core` 461, `tesserae-mods` 121). One step hides
whether compile or execution dominates, and the fix differs:

- Compile dominates: avoid compiling twice (the `Lint` job's clippy builds the
  same crates), or give our own crates a cheaper dev profile.
- Execution dominates: try `cargo-nextest` and look for slow outliers.

## What changes

1. "Run tests" becomes two steps: `cargo test ... --no-run`, then the same
   `cargo test ...`. The flags are identical, including the `opt-level=0`
   config, so the fingerprints match and the second step compiles nothing.
2. The comment above the step said "37 unit tests in well under a second",
   which is stale. It now states what the step runs.

## Result of the first run

Run on `def44a5`: Build tests 107s, Run tests 635s of an 814s job. Execution
dominates, and clippy in `Lint` takes 11s, so compiling twice is not the cost.

## Second step

`cargo-nextest` replaces `cargo test` for the unit tests, with `cargo test --doc`
kept for doctests. A `ci` profile writes JUnit XML, and a step turns the 25
slowest tests into `::notice` annotations, which the API serves without auth
where job logs and artifacts are not. The next change is chosen from those
names.

## Result of the second run

Nextest took the job from 814s to 679s (Run tests 635s to 508s). The ten
slowest tests, 35s to 191s each, are mostly `tesserae-core` property and
brute-force meshing and LOD checks, plus server tests that call them. They are
CPU-bound at `opt-level=1`.

## Third step

CI passes `--config 'profile.dev.package.tesserae-core.opt-level=3'` to the
build, run and doctest steps. It is a CLI flag, so local builds keep
`[profile.dev]`. The slow-test report becomes one annotation, since GitHub
shows 10 per step and cut the earlier list.

## Not in this change

Any speed-up. This change only makes the next run readable; the fix is chosen
from its numbers.

## Done when

- The run reports separate durations for the build and the run step.
- Those two numbers are recorded on the PR before any further change.
- The run's annotations list the slowest tests.
