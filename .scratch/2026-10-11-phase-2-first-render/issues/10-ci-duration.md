# Stop the Test job compiling Bevy twice

Status: done
Blocked by: 09

## Task

The `Test` job ran for 22 minutes while `Lint` took 5. The gap is not noise, and
neither is it the cold cache alone.

`cargo bench --workspace --no-run` builds a bench binary for **every** workspace
member. `tesserae-core` is the only crate with a `[[bench]]`, but cargo still
compiles `tesserae`'s binary in the bench profile, which inherits release — so
the job compiled the whole Bevy tree a second time, at release optimisation,
having already compiled it for the tests. Measured: `--workspace` compiles the
app; `--workspace --exclude tesserae` finishes in 0.11s and still builds
`tesserae-core`'s benches.

Separately, `[profile.dev.package."*"] opt-level = 3` in the workspace manifest
puts every one of Bevy's ~600 dependencies through LLVM at -O3. That setting
earns its place locally — without it a debug Bevy app runs an order of magnitude
too slow — but CI never runs the app. It runs 37 unit tests in under a second.

`Lint` is cheap for a reason that is not a bug: `cargo clippy` is check-shaped
and builds dependencies as metadata, skipping most codegen.

## Done when

The Test job builds the Bevy tree once rather than twice, and does it without
the -O3 pass over dependencies.

## Notes

The opt-level override is `--config 'profile.dev.package."*".opt-level=0'` on
the test invocation, not a change to the workspace manifest — local dev must
keep opt-level 3. Verified by reading the rustc command line: without the flag a
dependency gets `-C opt-level=3`, with it the flag is absent and rustc's default
0 applies. The glob matches dependencies only, so our own crates keep
`[profile.dev]`'s opt-level 1.

Not applied to `Lint`: clippy's dependencies are metadata-only, so it would
invalidate that job's cache for no gain.

`Swatinem/rust-cache` keys per job, so Lint, Test and Build each build the tree
independently. That is the action's design, not a misconfiguration. It also does
not save a cache when a job fails, which is why the earlier wayland failure left
nothing behind for the following run.

## Comments
