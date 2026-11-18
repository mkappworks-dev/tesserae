# 05: Determinism CI job on every release target

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 04.

## What to build

A `Determinism` job in CI runs only the generator digest tests, in a release
build, on every PR, on Linux, Windows, arm64 macOS and x86-64 macOS. x86-64
macOS builds for `x86_64-apple-darwin` and runs under Rosetta on the arm64
runner; if Rosetta is not available there, that target is dropped with a note in
the job saying why, and no self-hosted runner replaces it. The toolchain stays
unpinned `stable`.

The digests come from today's `std` maths, so a target may fail before 07 lands.
That failure is the measurement this work exists for. Record which cases differ
on which target in a comment here; failing targets continue on error with a note
naming 07, and the job is not a required check until 07.

## Acceptance criteria

- [ ] The job runs on every PR on the four targets (or three, with the Rosetta
      note).
- [ ] It runs the digest tests only, not the whole suite.
- [ ] Which digests differ on which target is recorded in a comment here.
- [ ] Every target allowed to fail names 07 as what removes the allowance.
