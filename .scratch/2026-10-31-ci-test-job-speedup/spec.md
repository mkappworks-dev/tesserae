# Speed up the CI Test job

Off the build order, so the branch is `tech/ci-test-job-speedup`. No tickets:
the change is confined to `.github/workflows/ci.yml`.

## Why

The `Test` job takes about 22 minutes; `Build` takes about 3, so the cache
works and a cold Bevy build is not the cause. Two things in `Test` are not
cached or are duplicated:

- `scripts/build-guests` runs 22 separate `cargo build --release` invocations
  (20 test guests, `demo`, `meadow`). Each is its own workspace root with its
  own `target/`, which `rust-cache` does not cover, so `wit-bindgen` and its
  macros recompile 22 times on every run.
- `cargo bench --no-run` rebuilds wasmtime and cranelift at opt-level 3 in the
  same job that `cargo test` already built them at opt-level 0. The two
  profiles share no artifacts.

## What changes

1. The guest-build step sets `CARGO_TARGET_DIR` to one directory under
   `target/`, so every guest shares compiled dependencies and `rust-cache`
   saves them. The script is unchanged: it already locates artifacts through
   `cargo metadata`, so local runs keep per-guest target directories.
2. `cargo bench --no-run` moves to its own `Benches` job, running in parallel
   with `Test`. The benches load guests at run time, not compile time, so the
   new job needs no guest build.

## What is given up

`Benches` is a new check, not yet in the required set listed in
`CONTRIBUTING.md`. Until it is added in repository settings, a bench that stops
compiling no longer blocks a merge. Folding it into a required check would
bring back the serialisation this change removes.

## Not in this change

A larger runner, `mold`, and a cheaper `Build` job on PRs: `Build` is not the
slow job.

## Done when

- `Test` wall clock on a PR with a warm `main` cache is measurably below the
  22 minutes it took before.
- `Test` and `Benches` both pass on the PR.
