# Add `crates/tesserae` as a Bevy 0.20 binary crate

Status: done
Blocked by: none

## Task

Create `crates/tesserae/` with `bevy = "0.20.0"` and a path dependency on
`tesserae-core`, and add it to `members` in the root `Cargo.toml`.

Verify the dev profile survived: the root manifest must still carry
`[profile.dev] opt-level = 1` and `[profile.dev.package."*"] opt-level = 3`.
Without them a debug Bevy build runs an order of magnitude slower than it
should, which reads as "the engine is slow" rather than "the build is
unoptimised".

## Done when

`cargo build -p tesserae` succeeds and `cargo test --workspace` visits the new
crate. A crate missing from `members` is silently skipped by a workspace test
run, so check that the crate's own tests actually appear in the output.

## Notes

Expect 5-15 minutes on the first build.

## Comments
