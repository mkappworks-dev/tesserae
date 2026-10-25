# 08: `cargo dev` loads the in-repo demo by default

Status: ready-for-agent
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

- [ ] In a `dev` build with the demo built and no directory given, the demo is
      the Game, with no Trust prompt and no Trust record written.
- [ ] In a `dev` build with the demo not built, the World opens with no Game and
      logs the build hint once.
- [ ] A directory given by argument or environment is used instead, with its
      normal Trust handling, in both builds.
- [ ] A release build never selects or trusts the in-repo demo on its own.
- [ ] Loader option tests cover all four cases without building a guest.
- [ ] README and CONTRIBUTING say that `cargo dev` runs the demo once the
      guests are built.
- [ ] `cargo fmt --all`, `npm run fmt` and `./scripts/cargo-ci` pass.
