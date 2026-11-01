# Bump Bevy 0.20.0 to 0.20.1

Status: done

Patch release. `cargo update` moves Bevy and its transitive crates; no source
change is expected. Bevy 0.20.1 depends on `ruzstd` 0.9.1, which clears
RUSTSEC-2026-0344, so the `deny.toml` ignore goes with this bump (see
`.scratch/2026-10-13-ruzstd-advisory/`).

## Done when

- `Cargo.lock` has `bevy` 0.20.1 and `ruzstd` >= 0.9.1.
- `deny.toml` has no `RUSTSEC-2026-0344` entry and `cargo deny check` passes.
- `./scripts/cargo-ci` passes.
