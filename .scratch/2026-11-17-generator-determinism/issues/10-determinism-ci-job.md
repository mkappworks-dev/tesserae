# 10: Determinism CI job on every release target

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: 09.

## What to build

A `Determinism` job in CI runs only the generator digest tests, in a release
build, on every PR, on Linux, Windows, arm64 macOS and x86-64 macOS. x86-64
macOS builds for `x86_64-apple-darwin` and runs under Rosetta on the arm64
runner; if Rosetta is not available there, that target is dropped with a note in
the job saying why, and no self-hosted runner replaces it. The toolchain stays
unpinned `stable`.

The digests come from today's `std` maths, so a target may fail before 12 lands.
That failure is the measurement this work exists for. Record which cases differ
on which target in a comment here; failing targets continue on error with a note
naming 12, and the job is not a required check until 12.

## Acceptance criteria

- [x] The job runs on every PR on the four targets (or three, with the Rosetta
      note).
- [x] It runs the digest tests only, not the whole suite.
- [x] Which digests differ on which target is recorded in a comment here.
- [x] Every target allowed to fail names 12 as what removes the allowance.

## Comments

- The job is its own workflow, `.github/workflows/determinism.yml`, with CI's triggers plus `workflow_dispatch`; in `ci.yml` the manual trigger would run the whole of CI. One matrix of four, `fail-fast: false`, each `continue-on-error` with a comment naming 12. Cached like CI (`Swatinem/rust-cache`, keyed by target, saved from `main` only).
- Rosetta is on the arm64 `macos-latest` image (macOS 26): its template, [`macOS-26.arm64.anka.pkr.hcl`](https://github.com/actions/runner-images/blob/main/images/macos/templates/macOS-26.arm64.anka.pkr.hcl), runs [`install-rosetta.sh`](https://github.com/actions/runner-images/blob/main/images/macos/scripts/build/install-rosetta.sh) (`softwareupdate --install-rosetta --agree-to-license`) unconditionally. The image's software list does not name it, as it names no OS component. So x86-64 macOS stays, built with `--target x86_64-apple-darwin`; a `Check Rosetta` step fails by name if a later image drops it.
- What it runs, in release: the whole of `generator_version`, `coarse_generation` and `surface_features` (the recorded digests and the asserts that their boxes hold what they digest), and `settlement_worldgen`'s `a_fixed_size_settlement_lays_out_exactly_as_before_ranges`, the one other recorded digest in `tesserae-core`. `worldgen.rs`'s `terrain_digest` tests compare seeds with each other, not with a recording, so they stay out. The exact commands passed on this Mac (arm64): 10, 2, 8 and 1 tests, about 11 s of test time. The x86-64 target could not be run here: this Mac has no Rosetta.
- Not yet seen on GitHub: nothing has been pushed, so which digests differ on which target (the third criterion) waits for the first run, and so does the first. Status stays `ready-for-agent` until that run is recorded here.

First run, PR #65 at 6c37d99e (Determinism run 44353061754): the `Digest
tests` step passed on all four targets (x86_64-unknown-linux-gnu,
x86_64-pc-windows-msvc, aarch64-apple-darwin, x86_64-apple-darwin under
Rosetta). No digest differs on any target under `std` maths. The allowance is
on the job, so step conclusions are real; no failure annotations. Ticket 12
still swaps to `libm`: today's agreement is not guaranteed across platforms or
Rust versions.
