# 08: `cargo dev` loads the in-repo demo by default

Status: done
Blocked by: 04

Spec: [decision 10](../spec.md).

## What to build

In a `dev` build only, when neither the command line nor the environment names
a mods directory and the demo's built output exists in the repository, the
loader uses that output. Its Mods are trusted for the run without a prompt and
without writing a Trust record. When the output does not exist, the World opens
with no Game and the log says to run the guest build script. An explicit
directory always wins. A release build's default stays the user's data
directory with the full Trust check.

## Acceptance criteria

- [x] In a `dev` build with the demo built and no directory given, the demo is
      the Game, with no Trust prompt and no Trust record written.
- [x] In a `dev` build with the demo not built, the World opens with no Game and
      logs the build hint once.
- [x] A directory given by argument or environment is used instead, with its
      normal Trust handling, in both builds.
- [x] A release build never selects or trusts the in-repo demo on its own.
- [x] Loader option tests cover all four cases without building a guest.
- [x] README and CONTRIBUTING say that `cargo dev` runs the demo once the
      guests are built.
- [x] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.

## Comments

### 2026-10-26: implementation

- **Where the demo is.** `server::mods::dev_demo` is
  `CARGO_MANIFEST_DIR/../../games/demo/dist`, where `./scripts/build-guests`
  copies it, behind `cfg!(feature = "dev")`; a default-features binary gets
  `None`. Fixed at compile time because a dev binary runs from its own
  checkout (each worktree finds its own demo) whatever the working directory.
  "Built" is the directory existing; a partial build is the loader's error.
- **Resolution.** `LoadOptions::resolve` takes the demo path as a fifth input
  and returns `Option`: `--mods`, then `TESSERAE_MODS_DIR`, with their normal
  Trust, always win (even when they name the demo directory); else, given a
  demo path, a built demo is chosen with `TrustMode::ThisRun`, and an unbuilt
  one is `None`, which `load_from_env` turns into a World with no Game and
  one line, "No Game: the demo is not built; run ./scripts/build-guests".
  With no demo path (release) the user data directory is the default, as
  before.
- **Trust scoping.** `ThisRun` is only ever produced by that default branch,
  so a named directory can never get it. It ignores `--trust-all` and
  `TESSERAE_TRUST_MODS` (both would write a record). Under it each Mod is
  admitted by the new `Trust::grant_for_run` in `tesserae-mods`, which adds
  the record in memory and never saves, so the host's own check before
  compiling still passes on exactly the hashed build. The log says "Trusting
  in-repo demo Mod `demo` for this run".
- **Tests** in `server/mods.rs`: the existing resolution test with the new
  input; the demo built (with and without `--trust-all` and the trust
  variable), not built, an argument, the environment and the demo's own
  path named explicitly (normal Trust), and no demo path (release); the
  `dev` build's demo path; and `ThisRun` loading a guest with no prompt and
  no `trust.json`, then leaving an existing record unchanged after a
  rebuild. The test's `not(dev)` arm only compiles in a default build;
  cargo-ci runs tests with `dev`, so the release case is covered by the
  resolution test's `None` input and the default-features build.
- README and CONTRIBUTING: `cargo dev` runs the demo once the guests are
  built, with the hint, the explicit override and the release default.
- `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass: tesserae
  368 (2 ignored), core 324, meshing 68, tesserae-mods 92 (3 ignored).
