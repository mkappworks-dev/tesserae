# A single command that runs everything CI runs

Status: done
Blocked by: 01

## Task

Ticket 01 put every command in the README, but running them still means four
invocations in the right order, and the order matters — a clippy failure after
a six-minute test run wastes the test run.

Add one script that runs all of them, fail-fast, in CI's order.

## Done when

One command runs rustfmt, Prettier, clippy, tests, the bench build, the
boundary check and cargo-deny, and stops at the first failure.

## Notes

Named `scripts/cargo-ci`, not `scripts/ci.sh`, because cargo resolves
`cargo <name>` to a `cargo-<name>` executable found on PATH. Put `scripts/` on
PATH and it is `cargo ci`; otherwise it is `./scripts/cargo-ci`. Verified both.

A `[alias]` entry cannot do this, which is worth recording because it looks like
it should. Cargo aliases expand to cargo subcommands and arguments only, with no
shell escape — `ci = "!./scripts/ci.sh"` fails with `no such subcommand`. The
`!` form is git's, not cargo's. An alias *can* point at an external subcommand
(`ci = "ci"` resolving to `cargo-ci` on PATH), but that is redundant: cargo
finds `cargo-ci` with no alias at all.

cargo-deny is skipped with a note rather than failing when it is not installed.
Installing it is a multi-minute build, and CI runs it on every PR regardless;
making it mandatory would push people away from running this script at all.

The script cannot catch Linux-only system-library failures — see ticket 09 in
`.scratch/2026-10-11-phase-2-first-render/`.

## Comments
