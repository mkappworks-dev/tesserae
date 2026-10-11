# Contributing to Tesserae

Thank you for your interest in contributing to Tesserae. This guide covers the
process and requirements for contributing to the project.

## Contributor License Agreement (CLA)

All contributors must sign the Contributor License Agreement before their first
pull request can be merged. The CLA is managed via the
[CLA Assistant](https://github.com/cla-assistant/cla-assistant) GitHub App and
will automatically prompt you when you open your first PR.

The full CLA text is available at [.github/CLA.md](.github/CLA.md).

**Key terms:**

- You retain copyright ownership of your contributions.
- You grant the Licensor a copyright license and a **patent license** (with
  defensive termination), plus the right to sublicense and to relicense your
  contribution under BSL 1.1, under the Apache-2.0 Change License, or under
  other terms including proprietary ones.
- The patent license is what lets the Licensor convey, at each Change Date, the
  patent grant that Apache-2.0 requires.
- The CLA is a one-time requirement per contributor.

## License

The Tesserae engine is licensed under the [Business Source License 1.1](LICENSE)
(SPDX `BUSL-1.1`), with Apache-2.0 as the Change License four years after each
version's first public release. All code contributions are accepted under the
same terms. By submitting a contribution, you agree that your work will be
licensed under BSL 1.1.

Two parts of the repo are deliberately **not** BSL, because they are templates
people copy:

| Path          | Licence    | Why                                                                                                                                                            |
| ------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `games/demo/` | CC0 or MIT | It is the starting point for every game built on Tesserae. BSL here would block the on-ramp. See [ADR-0004](docs/adr/0004-demo-game-in-repo-real-game-out.md). |
| `assets/`     | CC0        | Default and placeholder art must be reusable in proprietary games.                                                                                             |

BSL is **source-available, not open source** — it is not OSI-approved, and the
licence text says so itself. Please don't describe the project as open source.
Note also that BSL code cannot be mixed into a GPL-licensed project before its
Change Date, and that redistributing a derivative work does not let you
relicense it.

## Code of Conduct

We are committed to providing a welcoming and productive environment for
everyone. All participants are expected to:

- Be respectful and professional in all interactions.
- Provide constructive feedback and accept it graciously.
- Focus on what is best for the project and the community.
- Refrain from any form of harassment, discrimination, or personal attacks.

Violations may result in removal from the project at the maintainers'
discretion.

## How to Contribute

### Reporting Bugs

1. Search [existing issues](../../issues) to avoid duplicates.
2. Open a new issue with a clear, descriptive title.
3. Include steps to reproduce, expected behavior, actual behavior, and your
   environment (OS and version, GPU, `rustc --version`, Tesserae version or
   commit SHA).
4. For a rendering bug, attach a screenshot and the first 50 lines of log output
   — the adapter and backend Tesserae selected are in there.
5. For a crash, attach the panic message and `RUST_BACKTRACE=1` output.

### Suggesting Features

1. Search existing issues and discussions to check if the idea has already been
   proposed.
2. Open a new issue with the `enhancement` label.
3. Describe the use case, the proposed solution, and any alternatives you
   considered.
4. Check [DESIGN.md](DESIGN.md) first. Several things are deliberate non-goals —
   mobile, web, hardware ray tracing at 1.0, fine-grained destruction — and the
   "Rejected, with reasons" table explains why. Reopening one of those is fine,
   but argue against the recorded reason.

## Code Submission

We follow a standard fork-and-branch workflow:

1. **Fork** the repository to your GitHub account.
2. **Clone** your fork locally.
3. **Create a branch** from `main` named `<type>/<slug>`, where `type` is
   `feat`, `fix` (or `bug`), or `tech` — e.g.
   `feat/greedy-mesher-shaped-voxels`, `fix/chunk-boundary-face-culling`.
4. **Make your changes** in focused, incremental commits.
5. **Push** your branch to your fork.
6. **Open a pull request** against `main` in the upstream repository.

Before you start on anything non-trivial, read [CONTEXT.md](CONTEXT.md) for the
project's vocabulary and [docs/architecture.md](docs/architecture.md) for the
crate boundaries. Three invariants in particular are easy to break while writing
code that looks correct, and the PR template asks about each of them.

## Development Setup

**Requirements:**

- Rust 1.97 or later (stable). `rustup toolchain install stable`
- A GPU with Metal (macOS 11+), Vulkan (Linux) or DX12 (Windows). macOS is the
  primary development platform.
- For mod development: `rustup target add wasm32-wasip2`

No database, no code generation step, no asset pipeline to run.

Formatting is tool-enforced and checked in CI. Run both before pushing:

```bash
cargo fmt --all     # Rust, per rustfmt.toml
npm run fmt         # Markdown, JSON, YAML, per .prettierrc.json
```

`npm install` once to get Prettier; `package.json` exists only to pin its
version. Markdown wraps at 80 columns with `proseWrap: "always"`, so write a
paragraph as one unit and let Prettier break the lines.

**Quick start:**

```bash
cargo run -p tesserae
```

### The dev loop

Put this in your `Cargo.toml` (it is already in the committed workspace
manifest) — without it a debug build of a Bevy app runs an order of magnitude
slower than it should, which reads as "the engine is slow" rather than "the
build is unoptimised":

```toml
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

What each kind of change costs:

| you edit                                         | what happens                                                                  |
| ------------------------------------------------ | ----------------------------------------------------------------------------- |
| a `.wesl` / `.wgsl` shader, a texture, any asset | **hot-reloads in the running app** — no rebuild, no restart                   |
| a mod or the demo game under `games/`            | rebuild the `.wasm`, reload the mod — **no engine rebuild**                   |
| `tesserae` (the Bevy app)                        | rebuild, restart (~5–20s warm)                                                |
| `tesserae-core`                                  | rebuild everything downstream, restart — it is a dependency of the whole tree |

Asset hot-reloading needs Bevy's `file_watcher` feature, which is enabled in the
dev profile. Shader hot reload is the reason look packs are authored as WGSL
assets rather than compiled in: a pack author edits and sees the result
immediately.

### Working on `tesserae-core`

`tesserae-core` has no Bevy and no wgpu dependency
([invariant 3](docs/architecture.md)), which means it runs headless and fast:

```bash
cargo test -p tesserae-core        # no GPU, no window, seconds
cargo bench -p tesserae-core       # the mesher; the number that gates a change
```

If you change anything in the mesher, chunk storage, light propagation or fluid
simulation, run the bench and put the before/after numbers in the PR. CI builds
the benches but does not measure them — a regression is a human judgement.

**Keep it that way.** CI fails the `Boundaries` job if `tesserae-core` or
`tesserae-mods` acquires a `bevy` or `wgpu` edge, even transitively.

### Working on mods

```bash
cd games/demo
cargo build --target wasm32-wasip2 --release
```

The host API is deliberately **batched**: region operations and a command queue,
not per-voxel calls. Crossing the WASM boundary copies, so call frequency — not
throughput — is what costs. See [ADR-0001](docs/adr/0001-wasm-mod-boundary.md)
before adding a host function.

## Planning Workflow (Claude Code)

Planned work is tracked as markdown under `.scratch/` and driven by skills from
the [`mattpocock-skills`](https://github.com/mattpocock/skills) plugin. Using
Claude Code is optional; the files are plain markdown and can be written by
hand.

Install the plugin once:

```bash
claude plugin install mattpocock-skills@claude-plugins-official
```

Then work through the chain:

| Step | Command       | What it does                                                                      | Output                                         |
| ---- | ------------- | --------------------------------------------------------------------------------- | ---------------------------------------------- |
| 1    | `/to-spec`    | Turns the conversation into a spec                                                | `.scratch/<date>-<slug>/spec.md`               |
| 2    | `/to-tickets` | Splits the spec into tickets, each listing the tickets that block it              | `.scratch/<date>-<slug>/issues/<NN>-<slug>.md` |
| 3    | `/triage`     | Moves a ticket through the triage roles; use it on anything not already triaged   | `Status:` line updated                         |
| 4    | `/implement`  | Builds from a ticket, test-first via `/tdd` where it can; does not ask for a plan | Code and tests                                 |

- `/to-spec` and `/to-tickets` output is already `ready-for-agent`. Run
  `/triage` on anything else: a bug, a rough idea, or a ticket with no
  `Status:`.
- `/implement` takes only `ready-for-agent` tickets whose `Blocked by` tickets
  are all `done`.
- Commit spec and ticket files with the branch that writes them, and set
  `Status: done` in the same commit that finishes a ticket.
- Roles and file conventions:
  [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md) and
  [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).
- Glossary and ADR rules for agents:
  [`docs/agents/domain.md`](docs/agents/domain.md).

## Commit Conventions

Use conventional commit prefixes for all commit messages:

| Prefix      | Usage                                      |
| ----------- | ------------------------------------------ |
| `feat:`     | New feature                                |
| `fix:`      | Bug fix                                    |
| `docs:`     | Documentation changes                      |
| `refactor:` | Code restructuring without behavior change |
| `perf:`     | Performance improvement                    |
| `tech:`     | Technical work with no user-visible change |
| `test:`     | Adding or updating tests                   |
| `chore:`    | Build, CI, dependency, or tooling changes  |

**Examples:**

```
feat(core): merge coplanar full-cube faces in the greedy mesher
fix(core): cull faces across chunk boundaries using the padded neighbourhood
perf(core): pack voxels into u32 to halve palette table pressure
docs: record the client-server split as ADR-0002
```

Keep the subject line under 72 characters. Add a body for additional context
when necessary.

Comment policy: write a comment only for the _why_ the code can't show — a
hidden constraint, an invariant, a workaround, a surprise. One line by default.
Task and history context belongs in the commit message, where it stays true.

## Pull Request Process

1. **Title and description** — Write a clear, concise PR title in Conventional
   Commits form; it is enforced by
   [`pr-title-lint.yml`](.github/workflows/pr-title-lint.yml) and becomes the
   squash-merge commit message.
2. **Fill in the template** —
   [`.github/pull_request_template.md`](.github/pull_request_template.md). Tick
   what applies and strike through what does not (`- [ ] ~~item~~`) so a
   reviewer can tell "not applicable" from "forgot".
3. **Link related issues** — Reference issues with `Closes #123` or
   `Relates to #456`.
4. **CI must pass** — `Lint`, `Boundaries`, `Test` and `Build` are required
   checks.
5. **Maintainer review required** — At least one maintainer must approve before
   merge.
6. **Squash-merge preferred** — PRs are squash-merged to keep history linear.
7. **Respond to feedback** — Push additional commits rather than force-pushing,
   so reviewers can see incremental changes.

## Questions

If you have questions about contributing, open a [discussion](../../discussions)
or reach out in an existing issue thread. We are happy to help you get started.
