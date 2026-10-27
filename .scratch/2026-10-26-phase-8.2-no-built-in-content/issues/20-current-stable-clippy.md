# 20: Current stable Clippy pixel iteration

Status: done
Blocked by: 19

## What to fix

The user reported the PR's failing Lint job on 2026-10-27. GitHub Actions
uses Rust 1.99.1, while the cached local toolchain is 1.97.1. The newer
Clippy flags seven constant-size chunks_exact(4) pixel loops with
chunks_exact_to_as_chunks: two in texture alpha classification and five
in its texture tests and the demo catalogue tests.

Use as_chunks::<4>().0.iter() at the reported sites, preserving complete
RGBA pixel iteration and the existing trailing-byte behaviour. Keep the
warning policy and CI toolchain selection unchanged. No WIT, guest,
texture artwork or rendering behaviour changes are needed.

## Acceptance criteria

- [x] All seven flagged loops use fixed-array iteration.
- [x] Formatting and full local CI pass, with no lint suppression.
- [x] The fix is pushed to the feature PR for current-stable verification.

## Comments

Failing job:
https://github.com/mkappworks-dev/tesserae/actions/runs/40501548296/job/122180206631
The other remote jobs passed on bc8218c. Local full CI was green; the
compiler-version difference explains why that missed the new lint.

Implemented the suggested fixed-array iteration at all seven sites. The
API is available on the workspace's Rust 1.97 minimum and needs no feature
or lint allowance. Existing alpha classification and texture tests cover
the same complete RGBA pixels. No artwork or native UI changed, so the
approved visual captures remain valid.

Validation: cargo fmt --all, npm run fmt and ./scripts/cargo-ci pass locally
on Rust 1.97.1 with 902 tests and the five existing manual measurements
ignored. Current-stable GitHub verification will run on the pushed feature
head; the PR watcher will report its outcome. No CI warning is suppressed.
